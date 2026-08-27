use axum::{extract::State, Json};
use std::sync::{Arc, Mutex};
use sysinfo::Networks;
use crate::services::network_info::{get_network_info, NetworkInterfaceInfo};

pub type SharedNetworks = Arc<Mutex<Networks>>;

pub async fn network_info_handler(State(networks): State<SharedNetworks>) -> Json<Vec<NetworkInterfaceInfo>> {
    let mut networks_lock = networks.lock().unwrap();
    let info = get_network_info(&mut networks_lock);
    Json(info)
}
