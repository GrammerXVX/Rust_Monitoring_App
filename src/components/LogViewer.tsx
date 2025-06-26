import React, { useCallback, useEffect, useRef, useState } from 'react'
import { FixedSizeList as List, ListOnScrollProps } from 'react-window'
import AutoSizer from 'react-virtualized-auto-sizer'
import { LogEntry } from '../types'

interface LogViewerProps {
  logs: LogEntry[]
  isMonitoring: boolean
  toggleMonitoring: () => void
  clearLogs: () => void
}

const getLogColor = (level: string) => {
  switch (level.toUpperCase()) {
    case 'ERROR': return 'text-red-600'
    case 'WARNING': return 'text-yellow-600'
    case 'INFO': return 'text-green-600'
    case 'DEBUG': return 'text-blue-500'
    case 'TRACE': return 'text-purple-500'
    default: return 'text-gray-500'
  }
}

export function formatNumber(n: number) {
  return n.toLocaleString('en-US')
}

export function LogViewer({
  logs,
  isMonitoring,
  toggleMonitoring,
  clearLogs,
}: LogViewerProps) {
  const [count, setCount] = useState(0)

  useEffect(() => {
    setCount(logs.length)
  }, [logs])

  const [autoScroll, setAutoScroll] = useState(true)
  const containerRef = useRef<HTMLDivElement>(null)
  const listRef = useRef<List>(null)

  useEffect(() => {
    if (autoScroll && logs.length && listRef.current) {
      listRef.current.scrollToItem(logs.length - 1, 'end')
    }
  }, [logs, autoScroll])
  const LOG_LEVELS = ['ERROR', 'WARNING', 'INFO', 'DEBUG', 'TRACE'];
  const logStats = logs.reduce((acc, log) => {
    const lvl = log.level.toUpperCase();
    acc[lvl] = (acc[lvl] || 0) + 1;
    return acc;
  }, {} as Record<string, number>);
  const onScroll = (props: ListOnScrollProps) => {
    if (props.scrollUpdateWasRequested) return
    if (!containerRef.current) return

    const container = containerRef.current
    const isAtBottom = container.scrollHeight - container.scrollTop - container.clientHeight < 50

    // Только если пользователь прокручивает — отключаем авто-прокрутку
    if (!isAtBottom) {
      setAutoScroll(false)
    }
  }

  const Row = useCallback(
    ({ index, style }: { index: number; style: React.CSSProperties }) => {
      const log = logs[index]
      return (
        <div
          style={{ ...style, minWidth: '100%' }}
          className={`flex items-start px-4 py-2 ${getLogColor(log.level)}`}
        >
          <span className="text-win-text-secondary w-48 flex-shrink-0">
            {log.timestamp}
          </span>
          <span className="font-semibold w-20 flex-shrink-0">
            [{log.level}]
          </span>
          <span className="flex-1 whitespace-nowrap">
            {log.message}
          </span>
        </div>
      )
    },
    [logs]
  )

  return (
    <div className="flex flex-col h-full">
      {/* Панель управления */}
      <div className="p-4 border-b border-win-border flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold text-win-text flex items-baseline gap-2">
            Real-time Log Monitoring
            <span className="text-sm text-win-text-secondary">
              Displayed: {formatNumber(count)} {count === 1 ? 'line' : 'lines'}
              {/* Счётчики по типам */}
              {LOG_LEVELS.map(lvl =>
                logStats[lvl]
                  ? <span
                    key={lvl}
                    className={`ml-2 font-semibold ${getLogColor(lvl)}`}
                  >
                    {lvl.toUpperCase()}: {logStats[lvl]}
                  </span>
                  : null
              )}
            </span>
          </h2>
        </div>
        <div className="flex items-center gap-4">
          {/* Статус */}
          <div className="flex items-center gap-2">
            {isMonitoring ? (
              <span className="flex items-center gap-1 text-green-600">
                <span className="w-3 h-3 rounded-full bg-green-600 animate-pulse" />
                <span className="text-sm font-medium">Monitoring active</span>
              </span>
            ) : (
              <span className="flex items-center gap-1 text-red-600">
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  className="w-3 h-3"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                  strokeWidth="2"
                >
                  <path d="M6 18L18 6M6 6l12 12" />
                </svg>
                <span className="text-sm font-medium">Monitoring stopped</span>
              </span>
            )}
          </div>

          {/* Auto-scroll */}
          <button
            onClick={() => setAutoScroll((v) => !v)}
            className={`flex items-center gap-1 px-4 py-2 text-sm rounded ${autoScroll
              ? 'bg-win-control-active text-win-text'
              : 'bg-win-control hover:bg-win-control-hover text-win-text-secondary'
              }`}
          >
            {autoScroll && (
              <svg
                xmlns="http://www.w3.org/2000/svg"
                className="w-4 h-4 text-green-500"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                strokeWidth="2"
              >
                <path d="M5 13l4 4L19 7" />
              </svg>
            )}
            <span>Auto scroll</span>
          </button>

          <button
            onClick={clearLogs}
            className="px-4 py-2 bg-win-control hover:bg-win-control-hover text-win-text rounded border border-win-border text-sm"
          >
            Clear
          </button>

          <button
            onClick={toggleMonitoring}
            className={`px-4 py-2 text-sm rounded border ${isMonitoring
              ? 'bg-red-600 hover:bg-red-700 text-white border-red-700'
              : 'bg-win-control hover:bg-win-control-hover text-win-text border-win-border'
              }`}
          >
            {isMonitoring ? 'Stop' : 'Start'}
          </button>
        </div>
      </div>

      {/* Контейнер логов */}
      <div
        ref={containerRef}
        className="flex-1 overflow-auto overflow-x-auto bg-win-log font-mono max-h-[calc(100vh-250px)]"
      >
        {logs.length > 0 ? (
          <div className="min-w-full">
            <div className="h-[calc(100vh-300px)]">
              <AutoSizer>
                {({ height, width }) => (
                  <List
                    ref={listRef}
                    height={height}
                    width={width}
                    itemCount={logs.length}
                    itemSize={35}
                    overscanCount={10}
                    onScroll={onScroll}
                  >
                    {Row}
                  </List>
                )}
              </AutoSizer>
            </div>
          </div>
        ) : (
          <div className="p-4 text-center text-win-text-secondary h-full flex items-center justify-center">
            {isMonitoring ? 'Waiting for logs…' : 'No logs to display'}
          </div>
        )}
      </div>
    </div>
  )
}