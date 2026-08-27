use serde::{Deserialize, Serialize};
use sysinfo::Networks;

#[derive(Serialize, Deserialize, Debug)]
pub struct NetworkInterfaceInfo {
    pub name: String,
    pub mac_address: String,
    pub ip_networks: Vec<String>,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

pub fn get_network_info(networks: &mut Networks) -> Vec<NetworkInterfaceInfo> {
    networks.refresh(true);
    let mut interfaces = Vec::new();

    for (interface_name, data) in networks.iter() {
        let ip_networks: Vec<String> = data.ip_networks()
            .iter()
            .map(|ip| ip.to_string())
            .collect();

        interfaces.push(NetworkInterfaceInfo {
            name: interface_name.clone(),
            mac_address: data.mac_address().to_string(),
            ip_networks,
            rx_bytes: data.received(),
            tx_bytes: data.transmitted(),
        });
    }

    interfaces
}
