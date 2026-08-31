use serde::{Deserialize, Serialize};
use sysinfo::Networks;
use std::process::Command;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct NetworkInterfaceInfo {
    pub name: String,
    pub mac_address: String,
    pub ip_networks: Vec<String>,
    pub gateway: Option<String>,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

// Fungsi pembantu untuk mengambil tabel routing dan memetakan nama interface ke gateway-nya
fn get_gateways() -> HashMap<String, String> {
    let mut gateways = HashMap::new();
    // Jalankan `ip route` (hanya baris default yang kita cari)
    if let Ok(output) = Command::new("ip").arg("route").output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.starts_with("default via") {
                    // Contoh format: "default via 192.168.1.1 dev eth0 proto dhcp metric 100"
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    let mut gw_ip = None;
                    let mut iface_name = None;

                    let mut i = 0;
                    while i < parts.len() {
                        if parts[i] == "via" && i + 1 < parts.len() {
                            gw_ip = Some(parts[i+1].to_string());
                        } else if parts[i] == "dev" && i + 1 < parts.len() {
                            iface_name = Some(parts[i+1].to_string());
                        }
                        i += 1;
                    }

                    if let (Some(ip), Some(name)) = (gw_ip, iface_name) {
                        // Jika dalam 1 device ada banyak rute default, ambil yang pertama (priority tertinggi)
                        gateways.entry(name).or_insert(ip);
                    }
                }
            }
        }
    }
    gateways
}

pub fn get_network_info(networks: &mut Networks) -> Vec<NetworkInterfaceInfo> {
    networks.refresh(true);
    let mut interfaces = Vec::new();
    let gateways = get_gateways();

    for (interface_name, data) in networks.iter() {
        let ip_networks: Vec<String> = data.ip_networks()
            .iter()
            .map(|ip| ip.to_string())
            .collect();

        // Ambil gateway jika ada di map
        let gateway = gateways.get(interface_name.as_str()).cloned();

        interfaces.push(NetworkInterfaceInfo {
            name: interface_name.clone(),
            mac_address: data.mac_address().to_string(),
            ip_networks,
            gateway,
            rx_bytes: data.received(),
            tx_bytes: data.transmitted(),
        });
    }

    interfaces
}
