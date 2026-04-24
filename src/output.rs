use std::collections::HashMap;
use std::fmt::Write as _;

use crate::cli::{Config, OutputFormat};
use crate::process::{
    ProcessOutput, get_vmrss, get_vmrss_cpu_total, get_vmrss_io_total, get_vmrss_swap_total,
    get_vmrss_total,
};

pub fn display_processes(
    config: &Config,
    pids: &[i32],
    peak_memory: &mut HashMap<i32, f64>,
    peak_total: &mut HashMap<i32, f64>,
    last_io: &mut HashMap<i32, (f64, f64)>,
    elapsed: f64,
) {
    if config.format == OutputFormat::Json {
        let mut entries = Vec::new();
        for pid in pids {
            let processes = get_vmrss(
                config.io,
                config.peak,
                config.swap,
                *pid,
                peak_memory,
                last_io,
                elapsed,
            );
            if processes.is_empty() {
                continue;
            }

            let current_total = get_vmrss_total(&processes);
            let total_peak = peak_total.entry(*pid).or_insert(0.0);
            *total_peak = total_peak.max(current_total);
            entries.push(json_entry(config, *pid, &processes, *total_peak));
        }
        println!("[{}]", entries.join(","));
        return;
    }

    for (index, pid) in pids.iter().enumerate() {
        if index > 0 {
            println!();
        }

        let processes = get_vmrss(
            config.io,
            config.peak,
            config.swap,
            *pid,
            peak_memory,
            last_io,
            elapsed,
        );
        if processes.is_empty() {
            continue;
        }

        let current_total = get_vmrss_total(&processes);
        let total_peak = peak_total.entry(*pid).or_insert(0.0);
        *total_peak = total_peak.max(current_total);
        print_vmrss(config, *pid, &processes, *total_peak);
    }
}

fn print_vmrss(config: &Config, main_pid: i32, processes: &[ProcessOutput], peak_total: f64) {
    for process in processes {
        if config.children || process.pid == main_pid {
            let mut output = format!(
                "{:indent$}{}({}): {:.2} MB",
                "",
                process.name,
                process.pid,
                process.mem,
                indent = process.space
            );

            if config.peak && process.peak_mem > 0.0 {
                write!(output, " | peak: {:.2} MB", process.peak_mem).ok();
            }
            if config.cpu {
                write!(output, " | cpu: {:.1}%", process.cpu).ok();
            }
            if config.io {
                if config.monitor {
                    write!(
                        output,
                        " | io: r {:.1} KB/s w {:.1} KB/s",
                        process.read_rate, process.write_rate
                    )
                    .ok();
                } else {
                    write!(
                        output,
                        " | io: r {:.2} MB w {:.2} MB",
                        process.read_rate / 1024.0,
                        process.write_rate / 1024.0
                    )
                    .ok();
                }
            }
            if config.swap {
                write!(output, " | swap: {:.2} MB", process.swap).ok();
            }

            println!("{output}");
        }
    }

    let mut output = format!("total: {:.2} MB", get_vmrss_total(processes));
    if config.peak && peak_total > 0.0 {
        write!(output, " | peak: {:.2} MB", peak_total).ok();
    }
    if config.cpu {
        write!(output, " | cpu: {:.1}%", get_vmrss_cpu_total(processes)).ok();
    }
    if config.io {
        let (total_read, total_write) = get_vmrss_io_total(processes);
        if config.monitor {
            write!(
                output,
                " | io: r {:.1} KB/s w {:.1} KB/s",
                total_read, total_write
            )
            .ok();
        } else {
            write!(
                output,
                " | io: r {:.2} MB w {:.2} MB",
                total_read / 1024.0,
                total_write / 1024.0
            )
            .ok();
        }
    }
    if config.swap {
        write!(output, " | swap: {:.2} MB", get_vmrss_swap_total(processes)).ok();
    }
    println!("{output}");
}

fn json_entry(
    config: &Config,
    main_pid: i32,
    processes: &[ProcessOutput],
    peak_total: f64,
) -> String {
    let process_entries = processes
        .iter()
        .filter(|process| config.children || process.pid == main_pid)
        .map(|process| json_process(config, process))
        .collect::<Vec<_>>()
        .join(",");

    let mut entry = format!(
        "{{\"main_pid\":{},\"processes\":[{}],\"total_memory_mb\":{}",
        main_pid,
        process_entries,
        json_number(get_vmrss_total(processes))
    );

    if config.peak && peak_total > 0.0 {
        write!(entry, ",\"peak_total_mb\":{}", json_number(peak_total)).ok();
    }
    if config.cpu {
        write!(
            entry,
            ",\"total_cpu_percent\":{}",
            json_number(get_vmrss_cpu_total(processes))
        )
        .ok();
    }
    if config.io {
        let (total_read, total_write) = get_vmrss_io_total(processes);
        if config.monitor {
            write!(
                entry,
                ",\"total_read_kb_per_sec\":{},\"total_write_kb_per_sec\":{}",
                json_number(total_read),
                json_number(total_write)
            )
            .ok();
        } else {
            write!(
                entry,
                ",\"total_read_mb\":{},\"total_write_mb\":{}",
                json_number(total_read / 1024.0),
                json_number(total_write / 1024.0)
            )
            .ok();
        }
    }
    if config.swap {
        write!(
            entry,
            ",\"total_swap_mb\":{}",
            json_number(get_vmrss_swap_total(processes))
        )
        .ok();
    }
    entry.push('}');
    entry
}

fn json_process(config: &Config, process: &ProcessOutput) -> String {
    let mut entry = format!(
        "{{\"pid\":{},\"name\":\"{}\",\"memory_mb\":{}",
        process.pid,
        json_escape(&process.name),
        json_number(process.mem)
    );

    if config.peak && process.peak_mem > 0.0 {
        write!(
            entry,
            ",\"peak_memory_mb\":{}",
            json_number(process.peak_mem)
        )
        .ok();
    }
    if config.cpu {
        write!(entry, ",\"cpu_percent\":{}", json_number(process.cpu)).ok();
    }
    if config.io {
        if config.monitor {
            write!(
                entry,
                ",\"read_kb_per_sec\":{},\"write_kb_per_sec\":{}",
                json_number(process.read_rate),
                json_number(process.write_rate)
            )
            .ok();
        } else {
            write!(
                entry,
                ",\"read_mb\":{},\"write_mb\":{}",
                json_number(process.read_rate / 1024.0),
                json_number(process.write_rate / 1024.0)
            )
            .ok();
        }
    }
    if config.swap {
        write!(entry, ",\"swap_mb\":{}", json_number(process.swap)).ok();
    }
    entry.push('}');
    entry
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            value if value.is_control() => {
                write!(escaped, "\\u{:04x}", value as u32).ok();
            }
            value => escaped.push(value),
        }
    }
    escaped
}

fn json_number(value: f64) -> String {
    if value.is_finite() {
        format!("{value:.6}")
    } else {
        "0".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_json_strings() {
        assert_eq!(json_escape("a\"b\\c\n"), "a\\\"b\\\\c\\n");
    }
}
