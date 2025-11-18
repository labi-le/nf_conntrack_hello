use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::io::{BufRead, BufReader,  Cursor};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

const MIN_CLIENT_BYTES: u64 = 500;
const MAX_SERVER_BYTES: u64 = 200;
const POLL_INTERVAL_MS: u64 = 10_000;

fn print_ip(ip: u32) -> String {
    format!(
        "{}.{}.{}.{}",
        (ip >> 24) & 0xFF,
        (ip >> 16) & 0xFF,
        (ip >> 8) & 0xFF,
        ip & 0xFF
    )
}

fn parse_ipv4(s: &str) -> Option<u32> {
    let mut octets = [0u8; 4];
    let mut parts = s.split('.');
    for i in 0..4 {
        let part = parts.next()?;
        octets[i] = part.parse().ok()?;
    }
    Some(u32::from_be_bytes(octets))
}

fn parse_conntrack_output(line: &str) -> Option<(u32, u64, u64)> {
    // conntrack -L: ipv4 2 tcp 6 117 ESTABLISHED src=192.168.1.1 dst=8.8.8.8 sport=443 dport=443 packets=10 bytes=5000 [ASSURED]
    if !line.contains("dport=443") || !line.contains("tcp") {
        return None;
    }

    let mut dst_ip: Option<u32> = None;
    let mut bytes_first: Option<u64> = None;
    let mut bytes_last: Option<u64> = None;

    for part in line.split_whitespace() {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };

        match key {
            "dst" if dst_ip.is_none() => {
                dst_ip = parse_ipv4(value);
            }
            "bytes" => {
                if let Ok(b) = value.parse::<u64>() {
                    if bytes_first.is_none() {
                        bytes_first = Some(b);
                    }
                    bytes_last = Some(b);
                }
            }
            _ => {}
        }
    }

    Some((dst_ip?, bytes_first?, bytes_last?))
}

fn process_conntrack_line(
    line_str: &str,
    reported_ips: &mut HashSet<u32>,
) -> Option<(String, u64, u64)> {
    let (dst_ip, bytes_c2s, bytes_s2c) = parse_conntrack_output(line_str)?;

    if bytes_c2s < MIN_CLIENT_BYTES || bytes_s2c > MAX_SERVER_BYTES {
        return None;
    }

    if !reported_ips.insert(dst_ip) {
        return None;
    }

    let ip_str = print_ip(dst_ip);
    Some((ip_str, bytes_c2s, bytes_s2c))
}

fn add_ips_to_nft(table_name: &str, new_ips: &[String]) -> Result<(), Box<dyn Error>> {
    if new_ips.is_empty() {
        return Ok(());
    }

    let elements = new_ips.join(", ");
    let nft_arg = format!("{{ {} }}", elements);

    let status = Command::new("nft")
        .args(&["add", "element", "inet", "fw4", table_name, &nft_arg])
        .status()?;

    match status.success() {
        true => eprintln!(
            "added {} IPs to nft set {}: {}",
            new_ips.len(),
            table_name,
            elements
        ),
        false => eprintln!("failed to add IPs to nft set {}: {}", table_name, elements),
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let table_name = env::args()
        .nth(1)
        .ok_or("usage: nf_conntrack_hello <nft_set_name>")?;

    eprintln!(
        "nf_conntrack_hello (conntrack binary mode, MIN_C2S={}B, MAX_S2C={}B, nft set={})",
        MIN_CLIENT_BYTES, MAX_SERVER_BYTES, table_name
    );

    let mut reported_ips: HashSet<u32> = HashSet::new();

    loop {
        let mut new_ips = Vec::new();

        for (ip_str, bytes_c2s, bytes_s2c) in conntrack_stream(&mut reported_ips)? {
            println!(
                "{} (c2s={}B s2c={}B) -> queued for nft set {}",
                ip_str, bytes_c2s, bytes_s2c, table_name
            );
            new_ips.push(ip_str);
        }

        add_ips_to_nft(&table_name, &new_ips)?;

        thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
    }
}

fn conntrack_stream<'a>(
    reported_ips: &'a mut HashSet<u32>,
) -> Result<impl Iterator<Item = (String, u64, u64)> + 'a, Box<dyn Error>> {
    let output = Command::new("conntrack")
        .args(&["-L", "-p", "tcp"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()?;

    if !output.status.success() {
        return Err("conntrack failed".into());
    }

    let reader = BufReader::new(Cursor::new(output.stdout));

    Ok(
        reader
            .lines()
            .filter_map(move |line| {
                let line_str = line.ok()?;
                process_conntrack_line(&line_str, reported_ips)
            }),
    )
}