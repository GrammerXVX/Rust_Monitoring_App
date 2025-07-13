use crate::{
    models::system_info::{SystemInfo, ProcessDetail, LocalProcessInfo},
    state::system::SystemMonitorState,
    utils::log_parser::is_system_process,
};
use sysinfo::{CpuExt, PidExt, ProcessExt, SystemExt};
use systemstat::Platform;
use tauri::State;
use tokio::time;
use nvml_wrapper::{self as nvml};

#[tauri::command]
pub async fn get_system_info(
    state: State<'_, SystemMonitorState>,
    selected_pid: Option<u32>,
) -> Result<SystemInfo, String> {
    {
        let mut sys = state.sys.lock().unwrap();
        sys.refresh_all();
    }

    time::sleep(time::Duration::from_millis(300)).await;

    let (cpu_usage, total_memory, used_memory, cpu_name, processes, selected_process_detail) = {
        let mut sys = state.sys.lock().unwrap();
        sys.refresh_cpu();

        let cpu_usage = sys.global_cpu_info().cpu_usage();
        let total_memory = sys.total_memory();
        let used_memory = sys.used_memory();
        let cpu_name = sys.global_cpu_info().brand().to_string();

        let mut processes = Vec::new();
        let mut selected_process_detail = None;

        for (pid, process) in sys.processes() {
            let pid_value = pid.as_u32();
            let name = process.name().to_string();

            if is_system_process(pid_value, &name) {
                continue;
            }

            let process_info = LocalProcessInfo {
                pid: pid_value,
                name: name.clone(),
                cpu_usage: process.cpu_usage()/ (sys.cpus().len() as f32),
                memory: process.memory(),
            };

            processes.push(process_info);

            if Some(pid_value) == selected_pid {
                selected_process_detail = Some(ProcessDetail {
                    pid: pid_value,
                    name,
                    cpu_usage: process.cpu_usage() / (sys.cpus().len() as f32),
                    memory: process.memory(),
                    status: format!("{:?}", process.status()),
                    exe_path: Some(process.exe().to_string_lossy().to_string()),
                    command_line: Some(process.cmd().join(" ")),
                });
            }
        }

        processes.sort_by(|a, b| {
            b.cpu_usage
                .partial_cmp(&a.cpu_usage)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        (
            cpu_usage,
            total_memory,
            used_memory,
            cpu_name,
            processes,
            selected_process_detail,
        )
    };

    let cpu_temp = state.stats.cpu_temp().ok();

    let (gpu_name, gpu_temp, gpu_usage) = match &state.nvml {
        Some(nvml) => match nvml.device_by_index(0) {
            Ok(device) => {
                let name = device.name().ok();
                let temp = device
                    .temperature(nvml::enum_wrappers::device::TemperatureSensor::Gpu)
                    .ok();
                let usage = device.utilization_rates().map(|u| u.gpu).ok();
                (name, temp, usage)
            }
            Err(_) => (None, None, None),
        },
        None => (None, None, None),
    };

    Ok(SystemInfo {
        cpu_usage,
        total_memory,
        used_memory,
        cpu_temp,
        cpu_name: Some(cpu_name),
        gpu_name,
        gpu_temp,
        gpu_usage,
        processes,
        selected_process: selected_process_detail,
    })
}