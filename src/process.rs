use std::collections::{HashMap, HashSet};
use std::ffi::c_int;
use std::process::Command;

const RUSAGE_INFO_V0: c_int = 0;

#[repr(C)]
#[derive(Default)]
struct RusageInfoV0 {
    ri_uuid: [u8; 16],
    ri_user_time: u64,
    ri_system_time: u64,
    ri_pkg_idle_wkups: u64,
    ri_interrupt_wkups: u64,
    ri_pageins: u64,
    ri_wired_size: u64,
    ri_resident_size: u64,
    ri_phys_footprint: u64,
    ri_proc_start_abstime: u64,
    ri_proc_exit_abstime: u64,
    ri_child_user_time: u64,
    ri_child_system_time: u64,
    ri_child_pkg_idle_wkups: u64,
    ri_child_interrupt_wkups: u64,
    ri_child_pageins: u64,
    ri_child_elapsed_abstime: u64,
    ri_diskio_bytesread: u64,
    ri_diskio_byteswritten: u64,
}

unsafe extern "C" {
    fn proc_pid_rusage(pid: c_int, flavor: c_int, buffer: *mut RusageInfoV0) -> c_int;
}

#[derive(Clone, Debug)]
pub struct ProcessOutput {
    pub pid: i32,
    pub name: String,
    pub space: usize,
    pub mem: f64,
    pub swap: f64,
    pub cpu: f64,
    pub peak_mem: f64,
    pub read_rate: f64,
    pub write_rate: f64,
}

struct ProcessSnapshot {
    name: String,
    rss_mb: f64,
    cpu: f64,
}

pub fn get_vmrss(
    config_io: bool,
    main_pid: i32,
    peak_memory: &mut HashMap<i32, f64>,
    last_io: &mut HashMap<i32, (f64, f64)>,
    elapsed: f64,
) -> Vec<ProcessOutput> {
    let mut outputs = Vec::new();
    let mut stack = vec![(main_pid, 0)];

    while let Some((pid, space)) = stack.pop() {
        let Some(snapshot) = get_process_snapshot(pid) else {
            continue;
        };

        let (read_rate, write_rate) = if config_io {
            get_process_io_rate(pid, last_io, elapsed)
        } else {
            (0.0, 0.0)
        };

        let peak = peak_memory.entry(pid).or_insert(0.0);
        *peak = peak.max(snapshot.rss_mb);

        outputs.push(ProcessOutput {
            pid,
            name: snapshot.name,
            space,
            mem: snapshot.rss_mb,
            swap: 0.0,
            cpu: snapshot.cpu,
            peak_mem: *peak,
            read_rate,
            write_rate,
        });

        let mut children = get_process_children(pid);
        children.reverse();
        for child in children {
            stack.push((child, space + 2));
        }
    }

    outputs
}

pub fn get_process_children(pid: i32) -> Vec<i32> {
    let Ok(output) = Command::new("pgrep")
        .args(["-P", &pid.to_string()])
        .output()
    else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.trim().parse::<i32>().ok())
        .collect()
}

pub fn find_process_by_name(name: &str) -> Result<Vec<i32>, std::io::Error> {
    let output = Command::new("pgrep").args(["-i", name]).output()?;
    if !output.status.success() {
        return Ok(Vec::new());
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.trim().parse::<i32>().ok())
        .collect())
}

pub fn filter_root_processes(pids: &[i32]) -> Vec<i32> {
    let pid_set = pids.iter().copied().collect::<HashSet<_>>();
    let mut child_set = HashSet::new();

    for pid in pids {
        for child in get_process_children(*pid) {
            if pid_set.contains(&child) {
                child_set.insert(child);
            }
        }
    }

    pids.iter()
        .copied()
        .filter(|pid| !child_set.contains(pid))
        .collect()
}

pub fn get_vmrss_total(processes: &[ProcessOutput]) -> f64 {
    processes.iter().map(|process| process.mem).sum()
}

pub fn get_vmrss_swap_total(processes: &[ProcessOutput]) -> f64 {
    processes.iter().map(|process| process.swap).sum()
}

pub fn get_vmrss_cpu_total(processes: &[ProcessOutput]) -> f64 {
    processes.iter().map(|process| process.cpu).sum()
}

pub fn get_vmrss_io_total(processes: &[ProcessOutput]) -> (f64, f64) {
    processes.iter().fold((0.0, 0.0), |(read, write), process| {
        (read + process.read_rate, write + process.write_rate)
    })
}

fn get_process_snapshot(pid: i32) -> Option<ProcessSnapshot> {
    let output = Command::new("ps")
        .args([
            "-p",
            &pid.to_string(),
            "-o",
            "pid=",
            "-o",
            "rss=",
            "-o",
            "%cpu=",
            "-o",
            "comm=",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().find(|line| !line.trim().is_empty())?;
    let mut fields = line.split_whitespace();
    let parsed_pid = fields.next()?.parse::<i32>().ok()?;
    let rss_kb = fields.next()?.parse::<f64>().ok()?;
    let cpu = fields.next()?.parse::<f64>().ok()?;
    let name = fields.collect::<Vec<_>>().join(" ");

    Some(ProcessSnapshot {
        name: if name.is_empty() {
            parsed_pid.to_string()
        } else {
            name
        },
        rss_mb: rss_kb / 1024.0,
        cpu,
    })
}

fn get_process_io_rate(
    pid: i32,
    last_io: &mut HashMap<i32, (f64, f64)>,
    elapsed: f64,
) -> (f64, f64) {
    let Some((read_bytes, write_bytes)) = get_process_io_bytes(pid) else {
        return (0.0, 0.0);
    };

    if elapsed == 0.0 {
        last_io.insert(pid, (read_bytes, write_bytes));
        return (read_bytes / 1024.0, write_bytes / 1024.0);
    }

    let previous = last_io.insert(pid, (read_bytes, write_bytes));
    let Some((last_read, last_write)) = previous else {
        return (0.0, 0.0);
    };

    (
        ((read_bytes - last_read) / elapsed / 1024.0).max(0.0),
        ((write_bytes - last_write) / elapsed / 1024.0).max(0.0),
    )
}

fn get_process_io_bytes(pid: i32) -> Option<(f64, f64)> {
    let mut info = RusageInfoV0::default();
    let result = unsafe { proc_pid_rusage(pid, RUSAGE_INFO_V0, &mut info) };
    if result == 0 {
        Some((
            info.ri_diskio_bytesread as f64,
            info.ri_diskio_byteswritten as f64,
        ))
    } else {
        None
    }
}
