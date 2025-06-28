use sysinfo::System;
use std::sync::{Arc, Mutex};

pub struct SystemMonitorState {
    pub sys: Mutex<System>,
    pub stats: systemstat::System,
    pub nvml: Option<Arc<nvml_wrapper::Nvml>>,
}