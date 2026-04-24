use std::collections::{HashMap, HashSet};
use std::ffi::c_int;
use std::process::Command;

const RUSAGE_INFO_V4: c_int = 4;

#[repr(C)]
#[derive(Default)]
struct RusageInfoV4 {
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
    ri_cpu_time_qos_default: u64,
    ri_cpu_time_qos_maintenance: u64,
    ri_cpu_time_qos_background: u64,
    ri_cpu_time_qos_utility: u64,
    ri_cpu_time_qos_legacy: u64,
    ri_cpu_time_qos_user_initiated: u64,
    ri_cpu_time_qos_user_interactive: u64,
    ri_billed_system_time: u64,
    ri_serviced_system_time: u64,
    ri_logical_writes: u64,
    ri_lifetime_max_phys_footprint: u64,
    ri_instructions: u64,
    ri_cycles: u64,
    ri_billed_energy: u64,
    ri_serviced_energy: u64,
    ri_interval_max_phys_footprint: u64,
    ri_runnable_time: u64,
}

unsafe extern "C" {
    fn proc_pid_rusage(pid: c_int, flavor: c_int, buffer: *mut RusageInfoV4) -> c_int;
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

struct ProcessUsage {
    read_bytes: f64,
    write_bytes: f64,
    lifetime_peak_mb: f64,
}

pub fn get_vmrss(
    config_io: bool,
    config_peak: bool,
    config_swap: bool,
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

        let usage = if config_io || config_peak {
            get_process_usage(pid)
        } else {
            None
        };

        let (read_rate, write_rate) = if config_io {
            get_process_io_rate(pid, usage.as_ref(), last_io, elapsed)
        } else {
            (0.0, 0.0)
        };

        let peak = peak_memory.entry(pid).or_insert(0.0);
        let current_peak = usage
            .as_ref()
            .map(|usage| usage.lifetime_peak_mb)
            .filter(|peak| *peak > 0.0)
            .unwrap_or(snapshot.rss_mb)
            .max(snapshot.rss_mb);
        *peak = peak.max(current_peak);

        outputs.push(ProcessOutput {
            pid,
            name: snapshot.name,
            space,
            mem: snapshot.rss_mb,
            swap: if config_swap {
                get_process_swap_mb(pid).unwrap_or(0.0)
            } else {
                0.0
            },
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
    usage: Option<&ProcessUsage>,
    last_io: &mut HashMap<i32, (f64, f64)>,
    elapsed: f64,
) -> (f64, f64) {
    let usage = match usage {
        Some(usage) => usage,
        None => {
            let Some(usage) = get_process_usage(pid) else {
                return (0.0, 0.0);
            };
            return get_process_io_rate_from_usage(pid, &usage, last_io, elapsed);
        }
    };

    get_process_io_rate_from_usage(pid, usage, last_io, elapsed)
}

fn get_process_io_rate_from_usage(
    pid: i32,
    usage: &ProcessUsage,
    last_io: &mut HashMap<i32, (f64, f64)>,
    elapsed: f64,
) -> (f64, f64) {
    let read_bytes = usage.read_bytes;
    let write_bytes = usage.write_bytes;

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

fn get_process_usage(pid: i32) -> Option<ProcessUsage> {
    let mut info = RusageInfoV4::default();
    let result = unsafe { proc_pid_rusage(pid, RUSAGE_INFO_V4, &mut info) };
    if result == 0 {
        Some(ProcessUsage {
            read_bytes: info.ri_diskio_bytesread as f64,
            write_bytes: info.ri_diskio_byteswritten as f64,
            lifetime_peak_mb: bytes_to_mb(info.ri_lifetime_max_phys_footprint),
        })
    } else {
        None
    }
}

fn get_process_swap_mb(pid: i32) -> Option<f64> {
    let output = Command::new("vmmap")
        .args(["-summary", &pid.to_string()])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    parse_vmmap_swap_mb(&String::from_utf8_lossy(&output.stdout))
}

fn parse_vmmap_swap_mb(output: &str) -> Option<f64> {
    let total_line = output.lines().find(|line| {
        line.trim_start()
            .strip_prefix("TOTAL")
            .is_some_and(|rest| rest.starts_with(char::is_whitespace))
    })?;
    let swapped = total_line.split_whitespace().nth(4)?;
    parse_vmmap_size_mb(swapped)
}

fn parse_vmmap_size_mb(value: &str) -> Option<f64> {
    let number_end = value
        .find(|ch: char| !ch.is_ascii_digit() && ch != '.')
        .unwrap_or(value.len());
    let (amount, unit) = value.split_at(number_end);
    let amount = amount.parse::<f64>().ok()?;

    match unit {
        "B" => Some(amount / 1024.0 / 1024.0),
        "K" => Some(amount / 1024.0),
        "M" => Some(amount),
        "G" => Some(amount * 1024.0),
        _ => None,
    }
}

fn bytes_to_mb(bytes: u64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_vmmap_total_swapped_column() {
        let output = "
===========                     ======= ========    =====  =======
TOTAL                            837.8M   199.8M    1952K    12.5M
";
        assert_eq!(parse_vmmap_swap_mb(output), Some(12.5));
    }

    #[test]
    fn parses_vmmap_units() {
        assert_eq!(parse_vmmap_size_mb("0K"), Some(0.0));
        assert_eq!(parse_vmmap_size_mb("1024K"), Some(1.0));
        assert_eq!(parse_vmmap_size_mb("1.5G"), Some(1536.0));
    }
}
