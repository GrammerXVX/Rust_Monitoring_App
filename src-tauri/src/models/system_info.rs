use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct SystemInfo {
    pub cpu_usage: f32,
    pub total_memory: u64,
    pub used_memory: u64,
    pub cpu_temp: Option<f32>,
    pub cpu_name: Option<String>,
    pub gpu_name: Option<String>,
    pub gpu_temp: Option<u32>,
    pub gpu_usage: Option<u32>,
    pub processes: Vec<LocalProcessInfo>,
    pub selected_process: Option<ProcessDetail>,
}

#[derive(Serialize, Clone)]
pub struct ProcessDetail {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory: u64,
    pub status: String,
    pub exe_path: Option<String>,
    pub command_line: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct LocalProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory: u64,
}