use axum::{extract::State, Json};
use std::sync::{Arc, Mutex};
use sysinfo::System;
use crate::services::system_info::{get_system_info, SystemInfo};

pub type SharedSystem = Arc<Mutex<System>>;

pub async fn system_info_handler(State(sys): State<SharedSystem>) -> Json<SystemInfo> {
    let mut sys_lock = sys.lock().unwrap();
    let info = get_system_info(&mut sys_lock);
    Json(info)
}
