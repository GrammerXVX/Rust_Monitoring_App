import React, { useCallback, useEffect, useRef, useState } from 'react'
import { FixedSizeList as List, ListOnScrollProps } from 'react-window'
import AutoSizer from 'react-virtualized-auto-sizer'
import { LogEntry } from '../types'
import { animate, motion, useMotionValue, useTransform } from 'motion/react'

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
  const MAX_LOG_ENTRIES = 100_000;
  const linesPercent = Math.min((logs.length / MAX_LOG_ENTRIES) * 100, 100);

 
  const progressMotion = useMotionValue(linesPercent);
  useEffect(() => {
    animate(progressMotion, linesPercent, { duration: 0.3 });
  }, [linesPercent]);
  useEffect(() => {
    setCount(logs.length)
  }, [logs])

  const [autoScroll, setAutoScroll] = useState(true)
  const containerRef = useRef<HTMLDivElement>(null)
  const listRef = useRef<List>(null)
  const progress = Math.min(count / MAX_LOG_ENTRIES, 1);

  // Для плавной анимации можно использовать motion value:
  const value = useMotionValue(0);
  const ref = useRef(progress);

  useEffect(() => {
    animate(value, progress, { duration: 0.6, ease: "easeInOut" });
    ref.current = progress;
  }, [progress]);

const barColor = useTransform(
  value,
  [0, 0.6, 0.7, 1],
  ["#008000", "#FFFF00", "#FFA500", "#FF0000"] // зелёный-жёлтый-оранжевый-красный
);
const width = useTransform(value, v => `${Math.round(v * 100)}%`);
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
          <div className="flex flex-col gap-2 w-full max-w-xs mt-2">
          <div className="flex items-center gap-2">
            <span className="text-sm text-win-text-secondary">
              {count.toLocaleString()} / {MAX_LOG_ENTRIES.toLocaleString()} lines
            </span>
            {progress > 0.85 && (
              <span className="ml-2 text-xs font-semibold" style={{ color: "#ef4444" }}>
                {progress >= 1 ? "Limit reached!" : "Approaching limit"}
              </span>
            )}
          </div>
          <div className="w-full h-3 bg-win-control rounded-full relative overflow-hidden">
            <motion.div
              className="h-3 rounded-full absolute left-0 top-0"
              style={{
                width,
                background: barColor,
              }}
            />
            {/* Маркеры лимитов */}
            {[0.75, 0.9, 1].map((val, i) => (
              <div
                key={i}
                className="absolute top-0 h-3 w-1"
                style={{
                  left: `${val * 100}%`,
                  background:
                    i === 2
                      ? "#ef4444"
                      : i === 1
                        ? "#FF0000"
                        : "#FFA500",
                  opacity: 0.7,
                  borderRadius: 1,
                }}
              />
            ))}
          </div>
        </div>
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
          <motion.button
            whileHover={{ scale: 1.05, boxShadow: "0 4px 24px rgba(0,0,0,0.10)" }}
            whileTap={{ scale: 0.95 }}
            transition={{ type: "spring", stiffness: 350, damping: 18 }}
            className="w-15 h-10 px-4 py-2 bg-win-control text-win-text rounded border border-win-border"
            onClick={clearLogs}
          >
            Clear
          </motion.button>
          <motion.button
            whileHover={{ scale: 1.05, boxShadow: "0 14px 24px rgba(0,0,0,0.10)" }}
            whileTap={{ scale: 0.95 }}
            transition={{ type: "spring", stiffness: 350, damping: 18 }}
            className={`w-15 h-10 px-4 py-2 text-sm rounded border ${isMonitoring
              ? 'bg-red-600 hover:bg-red-700 text-white border-red-700'
              : 'bg-win-control hover:bg-win-control-hover text-win-text border-win-border'
              }`}
            onClick={toggleMonitoring}
          >
            {isMonitoring ? 'Stop' : 'Start'}
          </motion.button>

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