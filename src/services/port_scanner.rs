use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PortInfo {
    pub protocol: String,
    pub local_address: String,
    pub port: String,           // Port number saja: "8080"
    pub scope: String,          // "public" | "local"
    pub state: String,
    pub process_name: String,   // Nama proses bersih: "sshd"
    pub pid: Option<String>,    // PID proses: "4761"
    pub process: String,        // Raw string untuk kompatibilitas
}

/// Parse nama proses dan PID dari string `users:(("sshd",pid=4761,fd=3))`
fn parse_process(raw: &str) -> (String, Option<String>) {
    // Ekstrak nama proses: cari string dalam tanda kutip
    let name = if let Some(start) = raw.find('"') {
        let rest = &raw[start + 1..];
        if let Some(end) = rest.find('"') {
            rest[..end].to_string()
        } else {
            raw.to_string()
        }
    } else {
        raw.to_string()
    };

    // Ekstrak PID: cari pola `pid=1234`
    let pid = if let Some(pid_pos) = raw.find("pid=") {
        let after_pid = &raw[pid_pos + 4..];
        let pid_str: String = after_pid.chars().take_while(|c| c.is_ascii_digit()).collect();
        if pid_str.is_empty() { None } else { Some(pid_str) }
    } else {
        None
    };

    (name, pid)
}

/// Tentukan scope dari local_address:
/// - "public"  jika 0.0.0.0, [::]., *, atau IP spesifik non-loopback
/// - "local"   jika 127.0.0.x, [::1], atau 127.0.0.54
fn parse_scope_and_port(local_address: &str) -> (String, String) {
    // Ekstrak port dari string address "0.0.0.0:8080" atau "[::]:8080" atau "*:1716"
    let port = if let Some(colon_pos) = local_address.rfind(':') {
        local_address[colon_pos + 1..].to_string()
    } else {
        local_address.to_string()
    };

    // Tentukan scope berdasarkan address
    let addr_lower = local_address.to_lowercase();
    let scope = if addr_lower.contains("127.0.0") || addr_lower.contains("[::1]") || addr_lower.contains("localhost") {
        "local".to_string()
    } else {
        "public".to_string()
    };

    (scope, port)
}

pub fn get_listening_ports() -> Vec<PortInfo> {
    let mut ports = Vec::new();

    if let Ok(output) = Command::new("ss").args(["-tulnp"]).output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);

            for line in stdout.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();

                if parts.len() >= 5 {
                    let protocol = parts[0].to_string();

                    if !protocol.starts_with("tcp") && !protocol.starts_with("udp") {
                        continue;
                    }

                    let state = parts[1].to_string();
                    let local_address = parts[4].to_string();

                    // Parse scope dan port dari local_address
                    let (scope, port) = parse_scope_and_port(&local_address);

                    // Parse process string
                    let raw_process = if parts.len() >= 7 {
                        parts[6..].join(" ")
                    } else {
                        String::new()
                    };

                    let (process_name, pid) = if raw_process.contains("users:") {
                        parse_process(&raw_process)
                    } else {
                        (String::new(), None)
                    };

                    ports.push(PortInfo {
                        protocol,
                        local_address,
                        port,
                        scope,
                        state,
                        process_name,
                        pid,
                        process: raw_process,
                    });
                }
            }
        }
    }

    // Sortir: Public di atas, lalu Local; dalam tiap grup: sortir berdasarkan port number
    ports.sort_by(|a, b| {
        // Public lebih dulu
        let scope_ord = a.scope.cmp(&b.scope).reverse(); // "public" < "local" alphabetically, reverse = public first
        if scope_ord != std::cmp::Ordering::Equal {
            return scope_ord;
        }
        // Dalam grup yang sama, sortir by port number
        let port_a: u32 = a.port.parse().unwrap_or(99999);
        let port_b: u32 = b.port.parse().unwrap_or(99999);
        port_a.cmp(&port_b)
    });

    ports
}
