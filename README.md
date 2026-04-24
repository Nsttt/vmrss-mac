# vmrss-mac

`vmrss` is a small macOS CLI for inspecting resident memory usage for a process
and its child process tree.

It accepts one or more process IDs or process-name patterns and prints RSS in MB.
Optional flags add CPU usage, observed peak memory, disk I/O, swap fields, JSON
output, and monitor mode.

## Install

With Homebrew, after publishing a tap repository named `homebrew-vmrss`:

```sh
brew tap Nsttt/vmrss
brew install vmrss
```

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

## Homebrew Tap

The formula lives in `Formula/vmrss.rb` and installs the prebuilt release
archive for the current Mac architecture.

To publish it as a tap:

```sh
gh repo create Nsttt/homebrew-vmrss --public --description "Homebrew tap for vmrss"
git clone https://github.com/Nsttt/homebrew-vmrss.git /tmp/homebrew-vmrss
mkdir -p /tmp/homebrew-vmrss/Formula
cp Formula/vmrss.rb /tmp/homebrew-vmrss/Formula/vmrss.rb
cd /tmp/homebrew-vmrss
git add Formula/vmrss.rb
git commit -m "feat: add vmrss formula"
git push origin main
```

Before publishing, validate through a local tap:

```sh
brew tap-new Nsttt/vmrss
cp Formula/vmrss.rb "$(brew --repo Nsttt/vmrss)/Formula/vmrss.rb"
brew audit --strict --online Nsttt/vmrss/vmrss
brew install Nsttt/vmrss/vmrss
brew test Nsttt/vmrss/vmrss
```

The formula currently uses `license :cannot_represent` because this repository
does not declare a project license yet. Add a `LICENSE` file and update the
formula with its SPDX identifier before publishing if you want the tap metadata
to show a concrete license.

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
