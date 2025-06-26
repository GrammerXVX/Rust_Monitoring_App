import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
interface SystemInfo {
    cpu_usage: number;
    total_memory: number;
    used_memory: number;
    cpu_temp?: number | null;
    cpu_name?: string | null;
    gpu_name?: string | null;
    gpu_temp?: number | null;
    gpu_usage?: number | null;
    processes: LocalProcessInfo[];
    selected_process?: ProcessDetail | null;
}

interface LocalProcessInfo {
    pid: number;
    name: string;
    cpu_usage: number;
    memory: number;
}


interface ProcessDetail {
    pid: number;
    name: string;
    cpu_usage: number;
    memory: number;
    status: string;
    exe_path?: string | null;
    command_line?: string | null;
}

export function SystemMonitor() {
    const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null);
    const [selectedPid, setSelectedPid] = useState<number | null>(null); 
    const [error, setError] = useState<string | null>(null);
    const [windowSize, setWindowSize] = useState({ width: 0, height: 0 });
    useEffect(() => {
        const updateSize = async () => {
            const win = getCurrentWindow();
            const size = await win.innerSize();
            setWindowSize({
                width: size.width,
                height: size.height
            });
        };

        updateSize();

        const unlisten = getCurrentWindow().onResized(() => {
            updateSize();
        });

        return () => {
            unlisten.then(f => f());
        };
    }, []);
    useEffect(() => {
        const fetchSystemInfo = async () => {
            try {
                const info = await invoke<SystemInfo>('get_system_info', {
                    selectedPid: selectedPid || undefined
                });
                setSystemInfo(info);
                setError(null);
            } catch (err) {
                setError('Failed to load system information');
                console.error('Error getting system info:', err);
            }
        };

        fetchSystemInfo();
        const interval = setInterval(fetchSystemInfo, 1000);
        return () => clearInterval(interval);
    }, [selectedPid]);
    const selectedProcess = systemInfo?.selected_process;
    const formatMemory = (bytes: number): string => {
        const units = ['B', 'KB', 'MB', 'GB'];
        let value = bytes;
        let unitIndex = 0;

        while (value >= 1024 && unitIndex < units.length - 1) {
            value /= 1024;
            unitIndex++;
        }

        return `${value.toFixed(unitIndex > 0 ? 1 : 0)} ${units[unitIndex]}`;
    };

    const formatTemp = (temp?: number | null): string => {
        return temp !== null && temp !== undefined ? `${temp.toFixed(1)}°C` : 'N/A';
    };

    if (error) {
        return (
            <div className="flex items-center justify-center h-full">
                <div className="text-red-500">{error}</div>
            </div>
        );
    }

    if (!systemInfo) {
        return (
            <div className="flex items-center justify-center h-full">
                <div className="text-win-text-secondary">Loading system information...</div>
            </div>
        );
    }

    return (
        <div className="flex flex-col h-full overflow-auto p-4 bg-win-log">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
                <div className="bg-win-bg-secondary p-3 rounded border border-win-border flex items-center mt-4">
                    <div className="flex-1">
                        <div className="text-xs text-win-text-secondary">Window Size</div>
                        <div className="text-sm text-win-text">
                            {windowSize.width} × {windowSize.height} px
                        </div>
                    </div>
                    <button
                        onClick={async () => {
                            const win = getCurrentWindow();
                            if (await win.isMaximized()) {
                                await win.unmaximize();
                            } else {
                                await win.maximize();
                            }
                        }}
                        className="p-1 rounded hover:bg-win-control-hover"
                        title="Toggle Maximize"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                            <path d="M8 3H5a2 2 0 0 0-2 2v3" />
                            <path d="M21 8V5a2 2 0 0 0-2-2h-3" />
                            <path d="M3 16v3a2 2 0 0 0 2 2h3" />
                            <path d="M16 21h3a2 2 0 0 0 2-2v-3" />
                        </svg>
                    </button>
                </div>
            </div>
            {/* CPU - Добавлено отображение названия */}
            <div className="bg-win-bg-secondary p-4 rounded border border-win-border">
                <h3 className="text-lg font-medium mb-2 text-win-text">
                    CPU: {systemInfo.cpu_name || 'Unknown'}
                </h3>
                <div className="flex justify-between items-center mb-1">
                    <span className="text-sm text-win-text-secondary">
                        Temperature: {formatTemp(systemInfo.cpu_temp)}
                    </span>
                    <span className="text-sm text-win-text-secondary font-medium">
                        {systemInfo.cpu_usage.toFixed(1) ?? " "}%
                    </span>
                </div>
                <div className="w-full bg-win-control rounded-full h-2.5">
                    <div
                        className="bg-blue-500 h-2.5 rounded-full"
                        style={{ width: `${systemInfo.cpu_usage}%` }}
                    ></div>
                </div>
            </div>

            {/* Memory */}
            <div className="bg-win-bg-secondary p-4 rounded border border-win-border">
                <h3 className="text-lg font-medium mb-2 text-win-text">Memory</h3>
                <div className="flex justify-between items-center mb-1">
                    <span className="text-sm text-win-text-secondary">
                        {formatMemory(systemInfo.used_memory)} / {formatMemory(systemInfo.total_memory)}
                    </span>
                    <span className="text-sm text-win-text-secondary font-medium">
                        {((systemInfo.used_memory / systemInfo.total_memory) * 100).toFixed(1) ?? " "}%
                    </span>
                </div>
                <div className="w-full bg-win-control rounded-full h-2.5">
                    <div
                        className="bg-green-500 h-2.5 rounded-full"
                        style={{
                            width: `${(systemInfo.used_memory / systemInfo.total_memory * 100)}%`
                        }}
                    ></div>
                </div>
            </div>

            {/* GPU */}
            {systemInfo.gpu_name && (
                <div className="bg-win-bg-secondary p-4 rounded border border-win-border md:col-span-2">
                    <h3 className="text-lg font-medium mb-2 text-win-text">GPU: {systemInfo.gpu_name}</h3>
                    <div className="grid grid-cols-2 gap-4">
                        <div>
                            <div className="flex justify-between items-center mb-1">
                                <span className="text-sm font-medium text-win-text-secondary">Usage</span>
                                <span className="text-sm text-win-text-secondary font-medium">
                                    {(systemInfo.gpu_usage || 0).toFixed(1) ?? " "}%
                                </span>
                            </div>
                            <div className="w-full bg-win-control rounded-full h-2.5">
                                <div
                                    className="bg-purple-500 h-2.5 rounded-full"
                                    style={{ width: `${systemInfo.gpu_usage || 0}%` }}
                                ></div>
                            </div>
                        </div>
                        <div>
                            <div className="flex justify-between items-center">
                                <span className="text-sm font-medium text-win-text-secondary">Temperature</span>
                                <span className="text-win-text font-medium">
                                    {formatTemp(systemInfo.gpu_temp) ?? " "}
                                </span>
                            </div>
                        </div>
                    </div>
                </div>
            )}


            {/* Выбор процесса */}
            <div className="mb-4">
                <label className="block text-sm font-medium mb-1 text-win-text">
                    Select Process:
                </label>
                <select
                    className="w-full px-3 py-2 rounded border bg-win-control border-win-border"
                    value={selectedPid || ''}
                    onChange={(e) => {
                        const pid = parseInt(e.target.value);
                        setSelectedPid(pid || null); 
                    }}
                >
                    <option value="">-- Select a process --</option>
                    {systemInfo.processes.map(process => (
                        <option key={process.pid} value={process.pid}>
                            {process.name} (PID: {process.pid})
                        </option>
                    ))}
                </select>
            </div>

            {/* Детали выбранного процесса */}
            {
                selectedProcess && (
                    <div className="bg-win-bg-secondary p-4 rounded border border-win-border mb-4">
                        <h3 className="text-lg font-medium mb-3 text-win-text">Process Details</h3>
                        <div className="grid grid-cols-2 gap-4">
                            <div>
                                <h4 className="text-sm font-medium text-win-text-secondary">Name</h4>
                                <div className="text-win-text">{selectedProcess.name}</div>
                            </div>
                            <div>
                                <h4 className="text-sm font-medium text-win-text-secondary">PID</h4>
                                <div className="text-win-text">{selectedProcess.pid}</div>
                            </div>
                            <div>
                                <h4 className="text-sm font-medium text-win-text-secondary">CPU Usage</h4>
                                <div className="flex items-center">
                                    <div className="w-full bg-win-control rounded-full h-2.5">
                                        <div
                                            className="bg-blue-500 h-2.5 rounded-full"
                                            style={{ width: `${Math.min(selectedProcess.cpu_usage, 100)}%` }}
                                        ></div>
                                    </div>
                                    <span className="ml-2 text-win-text font-medium">
                                        {selectedProcess.cpu_usage.toFixed(1)}%
                                    </span>
                                </div>
                            </div>
                            <div>
                                <h4 className="text-sm font-medium text-win-text-secondary">Memory</h4>
                                <div className="text-win-text">{formatMemory(selectedProcess.memory)}</div>
                            </div>
                        </div>
                    </div>
                )
            }
        </div >
    );
}