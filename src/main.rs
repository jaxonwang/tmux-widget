use std::process;
use std::thread;
use std::time;
use sysinfo::{NetworkExt, NetworksExt, ProcessorExt, RefreshKind, System, SystemExt};

fn network_bytes() -> (u64, u64) {
    let refresh = RefreshKind::new().with_networks();
    let system = System::new_with_specifics(refresh);

    let networkref = system.networks();

    // dont check the local & docker interface
    let if_filter = |(name, _): &(&String, &sysinfo::NetworkData)| -> bool {
        !name.starts_with("lo") && !name.starts_with("docker") && !name.starts_with("bridge")
    };

    let received: u64 = networkref
        .iter()
        .filter(if_filter)
        .map(|(_, n)| n.total_received())
        .sum();
    let sent: u64 = networkref
        .iter()
        .filter(if_filter)
        .map(|(_, n)| n.total_transmitted())
        .sum();

    (sent, received)
}

fn network_bandwidth(cfg: &Config) -> String {
    let (first_up, first_down) = network_bytes();
    thread::sleep(cfg.interval);
    let (second_up, second_down) = network_bytes();
    let seconds = cfg.interval.as_secs();
    let up_bandwidth = second_up.wrapping_sub(first_up) / seconds;
    let down_bandwidth = second_down.wrapping_sub(first_down) / seconds;
    let (up_name, done_name) = if cfg.with_icons {
        (" ", " ")
    } else {
        ("UP: ", "DOWN: ")
    };
    let width = 6;
    let up_bandwidth = pretty_size(up_bandwidth, cfg.fix_length, width);
    let down_bandwidth = pretty_size(down_bandwidth, cfg.fix_length, width);
    format!("{up_name}{up_bandwidth:>width$}/s {done_name}{down_bandwidth:>width$}/s",)
}

// will try best to fix the value into max_width
fn max_width_float(v: f64, max_width: usize) -> String {
    let precision = max_width;
    let value = format!("{:.*}", precision, v);
    // remove trailing zeros, then remove trailing dot

    let valuesplit = value.split('.').collect::<Vec<_>>();
    let integer = valuesplit[0];
    // too large, keep integer only
    if integer.len() + 1 >= max_width {
        // 1 is for decimal dot
        return integer.to_string();
    }
    let mut value_str = &value[..max_width]; // get prefix to fix width
    value_str = value_str.trim_end_matches('0').trim_end_matches('.'); // remove trailing zeros then dot
    return value_str.to_string();
}

// accept bytes show string
fn pretty_size(s: u64, fix_length: bool, max_width: usize) -> String {
    let (value, unit) = match s {
        s if s < 1000 => (s as f64, "B"),
        s if s < 1000 * 1024 => (s as f64 / 1024.0, "KB"),
        s if s < 1000 * 1024 * 1024 => (s as f64 / 1024.0 / 1024.0, "MB"),
        s if s < 1000 * 1024 * 1024 * 1024 => (s as f64 / 1024.0 / 1024.0 / 1024.0, "GB"),
        _ => (s as f64 / 1024.0 / 1024.0 / 1024.0 / 1024.0, "TB"),
    };
    let value_str = if fix_length {
        let value_max_width = max_width - 2;
        max_width_float(value, value_max_width)
    } else {
        format!("{:2}", value)
    };

    format!("{value_str}{unit}")
}

fn mem(cfg: &Config) -> String {
    let refresh = RefreshKind::new().with_memory();
    let system = System::new_with_specifics(refresh);
    let total_mem = system.total_memory() * 1024;
    let used_mem = system.used_memory() * 1024;
    let total_swap = system.total_swap() * 1024;
    let used_swap = system.used_swap() * 1024;

    let (memory_show, swap_show) = if cfg.with_icons {
        (" ", "易")
    } else {
        ("MEM: ", "SWP: ")
    };
    let width = 6;
    format!(
        "{memory_show}{:>width$}/{:>width$} {swap_show}{:>width$}/{:>width$}",
        pretty_size(used_mem, cfg.fix_length, width),
        pretty_size(total_mem, cfg.fix_length, width),
        pretty_size(used_swap, cfg.fix_length, width),
        pretty_size(total_swap, cfg.fix_length, width)
    )
}

fn cpu(cfg: &Config) -> String {
    let refresh = RefreshKind::new().with_cpu();
    let mut system = System::new_with_specifics(refresh);
    thread::sleep(cfg.interval);
    system.refresh_cpu();
    let processors = system.processors();
    let processor_num = processors.len();
    let cpu_usage_avg: f32 =
        processors.iter().map(|p| p.cpu_usage()).sum::<f32>() / processor_num as f32;

    let cpu_show = if cfg.with_icons { " " } else { "CPU: " };
    format!("{cpu_show}{:>4}", max_width_float(cpu_usage_avg as f64, 4))
}

#[derive(Clone)]
struct Config {
    with_icons: bool,
    interval: time::Duration,
    fix_length: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            with_icons: false,
            interval: time::Duration::from_secs(1),
            fix_length: true,
        }
    }
}

fn main() {
    if !System::IS_SUPPORTED {
        eprintln!("This OS is not supported!");
        process::exit(1);
    }

    let mut cfg: Config = Default::default();
    let mut ops = Vec::<fn(&Config) -> String>::new();

    let mut args_iter = std::env::args().skip(1);
    while let Some(arg) = args_iter.next() {
        match arg.as_str() {
            "--net" => ops.push(network_bandwidth),
            "--cpu" => ops.push(cpu),
            "--mem" => ops.push(mem),
            "--with-icons" => cfg.with_icons = true,
            "--no-fix-length" => cfg.fix_length = false,
            "--interval" => {
                let interval_sec = args_iter
                    .next()
                    .expect("missing value for interval")
                    .parse::<u64>()
                    .expect("bad interval");
                cfg.interval = time::Duration::from_secs(interval_sec);
            }
            _ => {
                eprintln! {"unknown option: {}", arg}
                process::exit(1);
            }
        }
    }

    let mut threads = vec![];
    let mut outputs = vec![];
    for (i, op) in ops.into_iter().enumerate() {
        let localcfg = cfg.clone();
        threads.push(thread::spawn(move || (i, op(&localcfg))));
    }

    threads
        .into_iter()
        .for_each(|t| outputs.push(t.join().unwrap()));
    outputs.sort_by_key(|(i, _)| *i);
    let outputs: Vec<String> = outputs.into_iter().map(|(_, s)| s).collect();
    println!("{}", outputs.join(" "));
}
