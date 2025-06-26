import { useState, useEffect, useReducer } from 'react'
import { LogViewer } from './components/LogViewer'
import { SortedLogs } from './components/SortedLogs'
import { SystemMonitor } from './components/SystemMonitor';
import { FileSelector } from './components/FileSelector'
import { ThemeProvider, useTheme } from './components/ThemeProvider'
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event'
import { LogEntry } from './types'
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalSize } from '@tauri-apps/api/window'

function ThemeSelector() {
  const { theme, setTheme } = useTheme()
  return (
    <select
      value={theme}
      onChange={(e) => setTheme(e.target.value as 'system' | 'dark' | 'light')}
      className="px-3 py-1.5 text-sm rounded border bg-win-control hover:bg-win-control-hover border-win-border focus:outline-none focus:border-win-border-focus"
    >
      <option value="system">System Theme</option>
      <option value="dark">Dark Theme</option>
      <option value="light">Light Theme</option>
    </select>
  )
}
function WindowSizeControls() {
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [prevSize, setPrevSize] = useState<{ width: number; height: number } | null>(null);
  const setWindowSize = async (width: number, height: number) => {
    const win = getCurrentWindow();
    await win.setSize(new LogicalSize(width, height));
    win.center();
  };
  useEffect(() => {
    const checkFullscreen = async () => {
      setIsFullscreen(await getCurrentWindow().isFullscreen());
    };
    checkFullscreen();
  }, []);
  const toggleFullscreen = async () => {
    try {
      if (!isFullscreen) {
        const size = await getCurrentWindow().innerSize();
        setPrevSize({ width: size.width, height: size.height });

        await getCurrentWindow().setFullscreen(true);
        setIsFullscreen(true);
      } else {
        await getCurrentWindow().setFullscreen(false);

        if (prevSize) {
          await getCurrentWindow().setSize(new LogicalSize(prevSize.width, prevSize.height));
          await getCurrentWindow().center();
        }
        setIsFullscreen(false);
      }
    } catch (error) {
      console.error("Error toggling fullscreen:", error);
    }
  };

  return (
    <div className="flex items-center space-x-2">
      <button
        onClick={() => setWindowSize(800, 600)}
        className="px-2 py-1 text-xs rounded border bg-win-control hover:bg-win-control-hover border-win-border"
        title="Small (800x600)"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
          <line x1="15" y1="3" x2="15" y2="21" />
        </svg>
      </button>

      <button
        onClick={() => setWindowSize(1000, 700)}
        className="px-2 py-1 text-xs rounded border bg-win-control hover:bg-win-control-hover border-win-border"
        title="Medium (1000x700)"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
          <path d="M8 7v10" />
          <path d="M16 7v10" />
        </svg>
      </button>

      <button
        onClick={() => setWindowSize(1200, 800)}
        className="px-2 py-1 text-xs rounded border bg-win-control hover:bg-win-control-hover border-win-border"
        title="Large (1200x800)"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
          <path d="M3 15h18" />
        </svg>
      </button>

      <button
        onClick={toggleFullscreen}
        className="px-2 py-1 text-xs rounded border bg-win-control hover:bg-win-control-hover border-win-border"
        title="Toggle Maximize"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M8 3H5a2 2 0 0 0-2 2v3" />
          <path d="M21 8V5a2 2 0 0 0-2-2h-3" />
          <path d="M3 16v3a2 2 0 0 0 2 2h3" />
          <path d="M16 21h3a2 2 0 0 0 2-2v-3" />
        </svg>
      </button>
    </div>
  );
}
function logsReducer(state: LogEntry[], action: { type: string; payload?: LogEntry | LogEntry[] }) {
  switch (action.type) {
    case 'ADD_LOG':
      return [...state, action.payload as LogEntry];
    case 'ADD_BATCH':
      return [...state, ...(action.payload as LogEntry[])];
    case 'CLEAR_LOGS':
      return [];
    case 'SET_LOGS':
      return action.payload as LogEntry[];
    default:
      return state;
  }
}

function AppContent() {
  const clearLogs = async () => {
    dispatch({ type: 'CLEAR_LOGS' });
    setShouldReloadAll(true);
    setLoadedLines(0);
    setTotalLines(0);
    setLoadProgress(0);
    await invoke('set_current_file', { path: logFilePath });
  };

  const [pendingFilePath, setPendingFilePath] = useState<string | null>(null);
  const [showConfirmDialog, setShowConfirmDialog] = useState(false);
  const [shouldReloadAll, setShouldReloadAll] = useState(false);
  const [isMonitoring, setIsMonitoring] = useState(false);
  const [activeTab, setActiveTab] = useState('realtime');
  const [logs, dispatch] = useReducer(logsReducer, []);
  const [isLoading, setIsLoading] = useState(false);
  const [loadProgress, setLoadProgress] = useState(0);
  const [totalLines, setTotalLines] = useState(0);
  const [loadedLines, setLoadedLines] = useState(0);
  const MAX_LOG_ENTRIES = 100000;
  //const [logFilePath, /*setLogFilePath*/] = useState('C:/Users/Артём/Desktop/flog_0.4.4_windows_amd64/generated.log')
  //const [logFilePath, setLogFilePath] = useState('C:/Users/Артём/AppData/Roaming/.minecraft/logs/latest.log')
  const [logFilePath, setLogFilePath] = useState('C:/Users/Артём/Zomboid/coop-console.txt')

  useEffect(() => {
    if (logs.length > MAX_LOG_ENTRIES) {
      // Оставляем только последние MAX_LOG_ENTRIES записей
      dispatch({ type: 'SET_LOGS', payload: logs.slice(-MAX_LOG_ENTRIES) });
    }
  }, [logs]);
  useEffect(() => {
    const unlistenTruncate = listen('file_truncated', async () => {
      await clearLogs();
    });

    return () => {
      unlistenTruncate.then(f => f());
    };
  }, [clearLogs]);
  useEffect(() => {
    const fetchCurrentFile = async () => {
      try {
        const currentFile = await invoke<string>('get_current_file')
        if (currentFile) {
          setLogFilePath(currentFile)
        }
      } catch (error) {
        console.error('Error fetching current file:', error)
      }
    }

    fetchCurrentFile()
  }, [])
  useEffect(() => {
    let unsub: (() => void) | null = null;

    const setupListener = async () => {
      try {
        const unlisten = await listen('file_cleared', async () => {
          await clearLogs();
          console.log('Log file was cleared, clearing UI logs');
        });
        unsub = unlisten;
      } catch (error) {
        console.error('Error setting up file_cleared listener:', error);
      }
    };

    setupListener();

    return () => {
      if (unsub) unsub();
    };
  }, [clearLogs]); // Добавляем clearLogs в зависимости
  useEffect(() => {
    const seenLogs = new Set<string>();

    const handleNewLogs = (event: { payload: LogEntry[] }) => {
      const newLogs = (event.payload || []).filter((entry) => {
        const key = `${entry.timestamp}-${entry.level}-${entry.message}`;
        if (seenLogs.has(key)) return false;
        seenLogs.add(key);
        return true;
      });
      if (newLogs.length > 0) {
        dispatch({ type: 'ADD_BATCH', payload: newLogs });
      }
    };

    const unlistenPromises = [
      listen('new_logs_batch', handleNewLogs),

      listen('load_progress', (event) => {
        const progress = event.payload as { current: number, total: number };
        setLoadedLines(progress.current);
        setTotalLines(progress.total);
        setLoadProgress(Math.round((progress.current / progress.total) * 100));
      }),

      listen('loading_success', async () => {
        console.log("✅ [Frontend] loading_success — отключаю isLoading");
        setIsLoading(false); // 🟢 ОБЯЗАТЕЛЬНО
        try {
          await invoke('start_file_monitoring', { filePath: logFilePath });
          setIsMonitoring(true);
        } catch (err) {
          console.error("Error starting monitoring:", err);
        }
      }),

      listen('loading_cancelled', async () => {
        console.log("❌ [Frontend] loading_cancelled — отключаю isLoading");
        setIsLoading(false); // 🟢 ОБЯЗАТЕЛЬНО
        try {
          await invoke('stop_file_monitoring');
          setIsMonitoring(false);
          setLoadProgress(0);
          setTotalLines(0);
          setLoadedLines(0);
        } catch (err) {
          console.error("Error stopping monitoring after cancel:", err);
        }
        await clearLogs();
      }),

      listen('loading_error', async (event) => {
        const errorMessage = event.payload as string;
        console.error("❗ [Frontend] loading_error:", errorMessage);
        setIsLoading(false); // 🟢 ОБЯЗАТЕЛЬНО
        try {
          await invoke('stop_file_monitoring');
          setIsMonitoring(false);
        } catch (err) {
          console.error("Error stopping monitoring after error:", err);
        }
        alert(`Error loading file: ${errorMessage}`);
      }),
    ];

    const unsubs: (() => void)[] = [];

    Promise.all(unlistenPromises).then((fns) => {
      fns.forEach((fn) => unsubs.push(fn));
    });

    return () => {
      unsubs.forEach((fn) => fn());
    };
  }, [logFilePath]);
  const switchFile = async (path: string) => {
    if (isMonitoring) {
      await invoke('stop_file_monitoring');
      setIsMonitoring(false);
    }

    if (isLoading) {
      await invoke('cancel_file_loading');
      // событие `loading_cancelled` уже очистит состояние
    }

    setLogFilePath(path);
    await invoke('set_current_file', { path }); // <- ГАРАНТИЯ сброса стейта

    await clearLogs();
  }; // Add logFilePath and clearLogs to dependencies
  const handleFileSelect = async (path: string) => {
    if (isMonitoring || isLoading) {
      setPendingFilePath(path);
      setShowConfirmDialog(true);
    } else {
      await switchFile(path);
    }
    await startLoading();
  };
  const startLoading = async () => {
    if (!logFilePath || isLoading) return;
    console.log(
      "DEBUG: loadedLines =", loadedLines,
      "totalLines =", totalLines,
      "shouldReloadAll =", shouldReloadAll
    );

    const currentlyLoading = await invoke<boolean>('is_loading');
    if (currentlyLoading) {
      console.log("Already loading, skipping startLoading command.");
      return;
    }

    setIsLoading(true);
    setLoadProgress(0);
    setTotalLines(0);
    setLoadedLines(0);

    try {
      await invoke('start_file_loading', {
        filePath: logFilePath,
        reloadAll: shouldReloadAll || loadedLines === totalLines,
      });
      setShouldReloadAll(false);
    } catch (err) {
      console.error('Error invoking start_file_loading:', err);
      setIsLoading(false);
      alert(`Failed to start loading: ${err}`);
    }
  };

  const cancelLoading = async () => {
    console.log("Frontend: Cancel button clicked");
    try {
      const currentlyLoading = await invoke<boolean>('is_loading');
      console.log("Is loading:", currentlyLoading);
      if (currentlyLoading) {
        console.log("Invoking cancel_file_loading command");
        await invoke('cancel_file_loading');
        console.log("Cancel command completed");
        // The 'loading_cancelled' event listener will handle setting setIsLoading(false),
        // stopping monitoring, and clearing logs.
      } else {
        console.log("Loading not active, skipping cancel");
      }
    } catch (error) {
      console.error('Error cancelling loading:', error);
    }
  };
  const toggleMonitoring = async () => {
    if (!logFilePath) {
      alert('Please select a log file first')
      return
    }

    try {
      if (isMonitoring) {
        await invoke('stop_file_monitoring');
        setIsMonitoring(false);
      } else {
        await startLoading();
      }
      // setIsMonitoring(!isMonitoring) // This is now handled by the event listeners
    } catch (error) {
      console.error('Error toggling monitoring:', error)
    }
  }

  return (
    <div className="flex flex-col w-full min-h-screen bg-win-bg text-win-text">
      {/* Модальное окно загрузки */}
      {isLoading && (
        <div className="fixed inset-0 bg-black bg-opacity-70 flex items-center justify-center z-50">
          <div className="bg-win-bg p-6 rounded-lg shadow-xl w-96">
            <h3 className="text-lg font-medium mb-4 text-win-text">Loading Log File</h3>

            <div className="mb-4">
              <div className="flex justify-between text-sm text-win-text-secondary mb-1">
                <span>Progress: {loadProgress}%</span>
                <span>{loadedLines}/{totalLines} lines</span>
              </div>
              <div className="w-full bg-win-control rounded-full h-3">
                <div
                  className="bg-blue-500 h-3 rounded-full"
                  style={{ width: `${loadProgress}%` }}
                ></div>
              </div>
            </div>

            <div className="text-center">
              <button
                onClick={cancelLoading}
                className="px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700">
                Cancel Loading
              </button>
            </div>
          </div>
        </div>
      )}


      {showConfirmDialog && (
        <div className="fixed inset-0 bg-black bg-opacity-70 flex items-center justify-center z-50">
          <div className="bg-win-bg p-6 rounded-lg shadow-xl w-96 border border-win-border text-win-text">
            <h3 className="text-lg font-medium mb-4">Switch Log File?</h3>
            <p className="mb-4">Monitoring or loading is active. Do you want to stop it and load a new log file?</p>
            <div className="flex justify-end space-x-2">
              <button
                onClick={() => {
                  setShowConfirmDialog(false);
                  setPendingFilePath(null);
                }}
                className="px-4 py-2 rounded bg-win-control hover:bg-win-control-hover border border-win-border"
              >
                Cancel
              </button>
              <button
                onClick={async () => {
                  if (pendingFilePath) {
                    await switchFile(pendingFilePath);
                  }
                  setShowConfirmDialog(false);
                  setPendingFilePath(null);
                }}
                className="px-4 py-2 rounded bg-red-600 text-white hover:bg-red-700"
              >
                Yes, switch
              </button>
            </div>
          </div>
        </div>
      )}
      <header className="bg-win-header text-win-text p-3 flex items-center justify-between border-b border-win-border">
        <h1 className="text-lg font-semibold" data-tauri-drag-region>Game Log Monitor</h1>
        <div className="flex items-center space-x-3">
          <WindowSizeControls />
          <ThemeSelector />
        </div>
      </header>
      <main className="flex-1 p-4 flex flex-col w-full">
        <div className="p-2 border-b border-win-border flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <FileSelector onFileSelect={handleFileSelect} />
            {/* 🔘 Кнопка "Копировать логи" */}
            <button
              onClick={() => {
                const fullText = logs.map(log => `[${log.timestamp}] [${log.level}] ${log.message}`).join('\n');
                navigator.clipboard.writeText(fullText)
                  .then(() => console.log('Logs copied to clipboard'))
                  .catch((err) => console.error('Failed to copy logs:', err));
              }}
              className="px-3 py-1.5 text-sm rounded border bg-win-control hover:bg-win-control-hover border-win-border focus:outline-none focus:border-win-border-focus"
            >
              Copy Logs
            </button>
            <div className="text-sm truncate max-w-xs group relative">
              <span className="truncate inline-block max-w-full">
                Current file: {logFilePath.split('/').pop() || logFilePath}
              </span>
              {logFilePath && (
                <span className="absolute bottom-full left-0 mb-2 p-2 bg-black text-white text-xs rounded opacity-0 group-hover:opacity-100 transition-opacity z-10 w-96 break-words">
                  {logFilePath}
                </span>
              )}
            </div>
          </div>
        </div>
        <div className="bg-win-bg-secondary rounded border border-win-border flex-1 flex flex-col overflow-hidden">
          <div className="border-b border-win-border">
            <div className="flex">
              <button
                className={`px-4 py-2 ${activeTab === 'realtime' ? 'bg-win-bg-active border-b-2 border-win-accent font-medium' : 'hover:bg-win-bg-hover'}`}
                onClick={() => setActiveTab('realtime')}
              >
                Real-time Logs
              </button>
              <button
                className={`px-4 py-2 ${activeTab === 'sorted' ? 'bg-win-bg-active border-b-2 border-win-accent font-medium' : 'hover:bg-win-bg-hover'}`}
                onClick={() => setActiveTab('sorted')}
              >
                Sorted Logs
              </button>
              <button
                className={`px-4 py-2 ${activeTab === 'system' ? 'bg-win-bg-active border-b-2 border-win-accent font-medium' : 'hover:bg-win-bg-hover'}`}
                onClick={() => setActiveTab('system')}
              >
                System Monitor
              </button>
            </div>
          </div>
          <div className="flex-1 min-h-[400px] overflow-auto">
            {activeTab === 'realtime' ? (
              <LogViewer
                logs={logs}
                isMonitoring={isMonitoring}
                toggleMonitoring={toggleMonitoring}
                clearLogs={clearLogs} // Передаем функцию
              />
            ) : activeTab === 'sorted' ? (
              <SortedLogs logs={logs} />
            ) : activeTab === 'system' ? (
              <SystemMonitor />
            ) : null}
          </div>
        </div>
      </main>
    </div>
  )
}

export function App() {
  return (
    <ThemeProvider>
      <AppContent />
    </ThemeProvider>
  )
}


