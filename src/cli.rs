use std::env;
use std::process;
use std::time::Duration;

#[derive(Debug)]
pub struct Config {
    pub monitor: bool,
    pub children: bool,
    pub interval: Duration,
    pub timeout: Option<Duration>,
    pub swap: bool,
    pub cpu: bool,
    pub peak: bool,
    pub io: bool,
    pub format: OutputFormat,
    pub targets: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
}

pub fn parse_args() -> Result<Config, String> {
    let mut config = Config {
        monitor: false,
        children: true,
        interval: Duration::from_secs(1),
        timeout: None,
        swap: false,
        cpu: false,
        peak: false,
        io: false,
        format: OutputFormat::Text,
        targets: Vec::new(),
    };

    let mut args = env::args().skip(1).peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-m" => config.monitor = true,
            "-c" => {
                config.children = parse_optional_bool(args.peek()).is_none_or(|value| {
                    args.next();
                    value
                })
            }
            "-i" => {
                let value = args
                    .next()
                    .ok_or_else(|| "Missing value for -i".to_string())?;
                config.interval = parse_duration(&value)
                    .ok_or_else(|| format!("Invalid interval format: {value}"))?;
            }
            "-t" => {
                let value = args
                    .next()
                    .ok_or_else(|| "Missing value for -t".to_string())?;
                config.timeout = Some(
                    parse_duration(&value)
                        .ok_or_else(|| format!("Invalid timeout format: {value}"))?,
                );
            }
            "--swap" => config.swap = true,
            "--cpu" => config.cpu = true,
            "--peak" => config.peak = true,
            "--io" => config.io = true,
            "--format" => {
                let value = args
                    .next()
                    .ok_or_else(|| "Missing value for --format".to_string())?;
                config.format = parse_format(&value)?;
            }
            value if value.starts_with("--format=") => {
                config.format = parse_format(value.trim_start_matches("--format="))?;
            }
            "-h" | "--help" => usage_and_exit(0),
            value if value.starts_with('-') => return Err(format!("Unknown option: {value}")),
            value => config.targets.push(value.to_string()),
        }
    }

    Ok(config)
}

pub fn usage_and_exit(code: i32) -> ! {
    eprintln!("Usage: vmrss [options] <pid|name> [<pid|name>...]");
    eprintln!("  -m              Monitor process");
    eprintln!("  -c [true|false] Show child processes (default: true)");
    eprintln!("  -i <duration>   Interval (e.g., 500ms, 2s, 1m)");
    eprintln!("  -t <duration>   Quit after duration (e.g., 5s, 1m)");
    eprintln!("  --swap          Show swap memory (macOS reports 0 per process)");
    eprintln!("  --cpu           Show CPU usage");
    eprintln!("  --peak          Show peak memory observed by this run");
    eprintln!("  --io            Show disk I/O rates");
    eprintln!("  --format json   Output JSON");
    process::exit(code);
}

fn parse_format(value: &str) -> Result<OutputFormat, String> {
    match value {
        "" => Ok(OutputFormat::Text),
        "json" => Ok(OutputFormat::Json),
        _ => Err(format!("Unsupported format: {value}")),
    }
}

fn parse_optional_bool(next: Option<&String>) -> Option<bool> {
    match next.map(String::as_str) {
        Some("true") => Some(true),
        Some("false") => Some(false),
        _ => None,
    }
}

fn parse_duration(value: &str) -> Option<Duration> {
    let split = value
        .find(|ch: char| !ch.is_ascii_digit() && ch != '.')
        .unwrap_or(value.len());
    let (amount, unit) = value.split_at(split);
    let amount = amount.parse::<f64>().ok()?;
    let seconds = match unit {
        "ms" => amount / 1000.0,
        "s" | "" => amount,
        "m" => amount * 60.0,
        "h" => amount * 60.0 * 60.0,
        _ => return None,
    };

    if seconds.is_sign_negative() {
        None
    } else {
        Some(Duration::from_secs_f64(seconds))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_go_style_durations() {
        assert_eq!(parse_duration("500ms"), Some(Duration::from_millis(500)));
        assert_eq!(parse_duration("2s"), Some(Duration::from_secs(2)));
        assert_eq!(parse_duration("1m"), Some(Duration::from_secs(60)));
        assert_eq!(parse_duration("1h"), Some(Duration::from_secs(3600)));
        assert_eq!(parse_duration("bad"), None);
    }
}
