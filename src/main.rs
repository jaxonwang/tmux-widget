use std::process;
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

    (received, sent)
}

fn network_bandwidth(interval: time::Duration, with_icons: bool) {
    let (first_up, first_down) = network_bytes();
    std::thread::sleep(interval);
    let (second_up, second_down) = network_bytes();
    let seconds = interval.as_secs();
    let up_bandwidth = second_up.wrapping_sub(first_up) / seconds;
    let down_bandwidth = second_down.wrapping_sub(first_down) / seconds;
    let (up_name, done_name) = if with_icons {
        (" ", " ")
    } else {
        ("UP: ", "DOWN: ")
    };
    println!(
        "{up_name}{}/s  {done_name}{}/s",
        pretty_size(up_bandwidth),
        pretty_size(down_bandwidth)
    )
}

// accept bytes show string
fn pretty_size(s: u64) -> String {
    let (value, unit) = match s {
        s if s < 1024 => (s as f64, "B"),
        s if s < 1024 * 1024 => (s as f64 / 1024.0, "KB"),
        s if s < 1024 * 1024 * 1024 => (s as f64 / 1024.0 / 1024.0, "MB"),
        s if s < 1024 * 1024 * 1024 * 1024 => (s as f64 / 1024.0 / 1024.0 / 1024.0, "GB"),
        _ => (s as f64 / 1024.0 / 1024.0 / 1024.0 / 1024.0, "TB"),
    };
    // precision: 3
    let value = format!("{:.2}", value);
    // remove trailing zeros, then remove trailing dot
    let value = value.trim_end_matches('0').trim_end_matches('.');
    format!("{}{}", value, unit)
}

fn cpu_mem(with_icons: bool) {
    let refresh = RefreshKind::new().with_cpu().with_memory();
    let system = System::new_with_specifics(refresh);
    let processors = system.processors();
    let processor_num = processors.len();
    let cpu_usage_avg: f32 =
        processors.iter().map(|p| p.cpu_usage()).sum::<f32>() / processor_num as f32;
    let total_mem = system.total_memory() * 1024;
    let used_mem = system.used_memory() * 1024;
    let total_swap = system.total_swap() * 1024;
    let used_swap = system.used_swap() * 1024;

    let (cpu_show, memory_show, swap_show) = if with_icons {
        (" ", " ", "易")
    } else {
        ("CPU: ", "MEM: ", "SWP: ")
    };
    println!(
        "{cpu_show}{cpu_usage_avg:.2} {memory_show}{}/{} {swap_show}{}/{}",
        pretty_size(used_mem),
        pretty_size(total_mem),
        pretty_size(used_swap),
        pretty_size(total_swap)
    );
}

fn main() {
    if !System::IS_SUPPORTED {
        eprintln!("This OS is not supported!");
        process::exit(1);
    }
    let mut do_net = false;
    let mut do_cpu_men = false;
    let mut with_icons = false;
    let mut interval = time::Duration::from_secs(1);

    let mut args_iter = std::env::args().skip(1);
    while let Some(arg) = args_iter.next() {
        match arg.as_str() {
            "--net" => do_net = true,
            "--cpu-mem" => do_cpu_men = true,
            "--with-icons" => with_icons = true,
            "--interval" => {
                let interval_sec = args_iter
                    .next()
                    .expect("missing value for interval")
                    .parse::<u64>()
                    .expect("bad interval");
                interval = time::Duration::from_secs(interval_sec);
            }
            _ => {
                eprintln! {"unknown option: {}", arg}
                process::exit(1);
            }
        }
    }
    if do_net {
        network_bandwidth(interval, with_icons);
    }
    if do_cpu_men {
        cpu_mem(with_icons);
    }
}
