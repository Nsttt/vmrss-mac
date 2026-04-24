# vmrss-mac

`vmrss` is a small macOS CLI for inspecting resident memory usage for a process
and its child process tree.

It accepts one or more process IDs or process-name patterns and prints RSS in MB.
Optional flags add CPU usage, observed peak memory, disk I/O, swap fields, JSON
output, and monitor mode.

## Install

Download the latest macOS archive from the GitHub release assets:

- `vmrss-<tag>-macos-arm64.tar.gz` for Apple Silicon Macs
- `vmrss-<tag>-macos-x86_64.tar.gz` for Intel Macs

Then unpack and move the binary onto your PATH:

```sh
tar -xzf vmrss-<tag>-macos-arm64.tar.gz
chmod +x vmrss
mv vmrss ~/.local/bin/vmrss
```

## Build

```sh
cargo build --release
```

The binary is written to:

```sh
target/release/vmrss
```

## Usage

```sh
vmrss [options] <pid|name> [<pid|name>...]
```

Examples:

```sh
vmrss 12345
vmrss Safari --cpu --peak
vmrss -m -i 500ms -t 10s Terminal --cpu --io
vmrss --format json --cpu --io 12345
```

Options:

```text
-m              Monitor process
-c [true|false] Show child processes (default: true)
-i <duration>   Interval (e.g., 500ms, 2s, 1m)
-t <duration>   Quit after duration (e.g., 5s, 1m)
--swap          Show swap memory (macOS reports 0 per process)
--cpu           Show CPU usage
--peak          Show peak memory observed by this run
--io            Show disk I/O rates
--format json   Output JSON
```

## macOS Notes

macOS does not expose Linux `/proc`, so this implementation uses macOS/userland
process data:

- `ps` for RSS, CPU, and process command names
- `pgrep` for process lookup and child process discovery
- `proc_pid_rusage` for per-process disk I/O counters

`--peak` reports macOS' lifetime peak physical footprint for each process when
available, and the highest total RSS observed while this tool is running.
`--swap` reads the swapped column from `vmmap -summary`, so it can be slower and
may report `0.00 MB` when macOS cannot inspect the target process.
