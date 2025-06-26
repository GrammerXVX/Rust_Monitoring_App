use crate::models::{LocalProcessInfo, ProcessDetail, SystemInfo};
use nvml_wrapper as nvml;
use std::sync::Mutex;
use sysinfo::{CpuExt,  PidExt, ProcessExt,  System, SystemExt};
use systemstat::{Platform, System as Stats};
use tauri::State;

pub struct SystemMonitorState {
    pub sys: Mutex<System>,
    pub stats: Stats,
    pub nvml: Option<nvml::Nvml>,
}

fn is_system_process(pid: u32, name: &str) -> bool {
    if cfg!(target_os = "windows") {
        pid < 100
            || name == "System"
            || name == "Registry"
            || name.contains("svchost")
            || name.contains("dllhost")
    } else if cfg!(target_os = "linux") {
        pid < 1000
            || name == "systemd"
            || name == "kthreadd"
            || name == "ksoftirqd"
            || name.contains("rcu")
            || name == "migration"
    } else {
        false
    }
}

#[tauri::command]
pub async fn get_system_info(
    state: State<'_, SystemMonitorState>,
    selected_pid: Option<u32>,
) -> Result<SystemInfo, String> {
    {
        let mut sys = state.sys.lock().unwrap();
        sys.refresh_all();
    }
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

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

            processes.push(LocalProcessInfo {
                pid: pid_value,
                name: name.clone(),
                cpu_usage: process.cpu_usage(),
                memory: process.memory(),
            });

            if Some(pid_value) == selected_pid {
                selected_process_detail = Some(ProcessDetail {
                    pid: pid_value,
                    name,
                    cpu_usage: (process.cpu_usage() / (sys.cpus().len() as f32)),
                    memory: process.memory(),
                    status: format!("{:?}", process.status()),
                    exe_path: Some(process.exe().to_path_buf().to_string_lossy().to_string()),
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

    let (gpu_name, gpu_temp, gpu_usage) = if let Some(nvml) = &state.nvml {
        nvml.device_by_index(0)
            .map(|device| (device.name().ok(), device.temperature(nvml::enum_wrappers::device::TemperatureSensor::Gpu).ok(), device.utilization_rates().map(|u| u.gpu).ok()))
            .unwrap_or((None, None, None))
    } else {
        (None, None, None)
    };

    Ok(SystemInfo {
        cpu_usage, total_memory, used_memory, cpu_temp, gpu_name, gpu_temp, gpu_usage, processes,
        cpu_name: Some(cpu_name),
        selected_process: selected_process_detail,
    })
}