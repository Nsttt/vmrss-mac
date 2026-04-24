mod cli;
mod output;
mod process;

use std::collections::HashMap;
use std::process as std_process;
use std::thread;
use std::time::Instant;

use cli::{parse_args, usage_and_exit};
use output::display_processes;
use process::{filter_root_processes, find_process_by_name};

fn main() {
    let config = parse_args().unwrap_or_else(|err| {
        eprintln!("{err}");
        usage_and_exit(1);
    });

    if config.targets.is_empty() {
        usage_and_exit(1);
    }

    let mut pids = Vec::new();
    for target in &config.targets {
        match target.parse::<i32>() {
            Ok(pid) => pids.push(pid),
            Err(_) => match find_process_by_name(target) {
                Ok(found) if !found.is_empty() => {
                    pids.extend(filter_root_processes(&found));
                }
                _ => {
                    eprintln!("No process found with name: {target}");
                    std_process::exit(1);
                }
            },
        }
    }

    let mut peak_memory = HashMap::new();
    let mut peak_total = HashMap::new();
    let mut last_io = HashMap::new();
    let mut last_io_time = Instant::now();
    let stop_at = config.timeout.map(|timeout| Instant::now() + timeout);

    loop {
        let now = Instant::now();
        let elapsed = now.duration_since(last_io_time).as_secs_f64();
        display_processes(
            &config,
            &pids,
            &mut peak_memory,
            &mut peak_total,
            &mut last_io,
            elapsed,
        );
        last_io_time = now;

        if !config.monitor {
            break;
        }

        println!();
        if let Some(stop_at) = stop_at
            && Instant::now() >= stop_at
        {
            break;
        }
        thread::sleep(config.interval);
    }
}
