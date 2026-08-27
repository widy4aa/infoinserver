use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Serialize, Deserialize, Debug)]
pub struct PortInfo {
    pub protocol: String,
    pub local_address: String,
    pub state: String,
    pub process: String,
}

pub fn get_listening_ports() -> Vec<PortInfo> {
    let mut ports = Vec::new();

    // Menggunakan `ss -tulnp` lebih stabil dan memberikan info PID/Proses
    // ss -tulnp => tcp, udp, listening, numeric (no dns resolving), processes
    if let Ok(output) = Command::new("ss")
        .args(["-tulnp"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            // Skip the header line
            for line in stdout.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                
                // ss format usually: Netid State Recv-Q Send-Q Local-Address:Port Peer-Address:Port Process
                if parts.len() >= 5 {
                    let protocol = parts[0].to_string(); // tcp, udp, u_str, dll.
                    
                    // Kita hanya peduli tcp dan udp
                    if protocol.starts_with("tcp") || protocol.starts_with("udp") {
                        let state = parts[1].to_string();
                        let local_address = parts[4].to_string();
                        
                        let process = if parts.len() >= 7 {
                            parts[6..].join(" ")
                        } else {
                            "Unknown".to_string()
                        };

                        ports.push(PortInfo {
                            protocol,
                            state,
                            local_address,
                            process,
                        });
                    }
                }
            }
        }
    }

    ports
}
