import React, { useState, useMemo } from 'react'
import { AnimatePresence, motion } from "motion/react"
import { useTheme } from './ThemeProvider'

export interface ProcessInfo {
    pid: number;
    name: string;
    cpu_usage: number;
    memory: number;
}

interface ProcessTableModalProps {
    processes: ProcessInfo[];
    visible: boolean;
    onClose: () => void;
}
interface SortableThProps {
  label: string;
  active: boolean;
  order: SortOrder;
  onClick: () => void;
}
type SortOrder = 'asc' | 'desc' | undefined;
export const ProcessTableModal: React.FC<ProcessTableModalProps> = ({ processes, visible, onClose }) => {
    const [sortBy, setSortBy] = useState<'pid' | 'name' | 'cpu_usage' | 'memory'>('pid')
    const [sortOrder, setSortOrder] = useState<'asc' | 'desc'>('asc')
    const [searchOpen, setSearchOpen] = useState(false)
    const [search, setSearch] = useState('')
    const { currentTheme } = useTheme()
    
    const filteredProcesses = useMemo(() => {
        let arr = [...processes]
        if (search) {
            const q = search.toLowerCase()
            arr = arr.filter(
                p =>
                    p.pid.toString().includes(q) ||
                    p.name.toLowerCase().includes(q)
            )
        }
        arr.sort((a, b) => {
            let res = 0
            if (sortBy === 'name') {
                res = a.name.localeCompare(b.name)
            } else if (sortBy === 'cpu_usage') {
                res = a.cpu_usage - b.cpu_usage
            } else if (sortBy === 'memory') {
                res = a.memory - b.memory
            } else {
                res = a.pid - b.pid
            }
            return sortOrder === 'asc' ? res : -res
        })
        return arr
    }, [processes, sortBy, sortOrder, search])
    const handleSort = (key: 'pid' | 'name' | 'cpu_usage' | 'memory') => {
        if (sortBy === key) setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc')
        else {
            setSortBy(key);
            setSortOrder('asc')
        }
    }



    if (!visible) return null;

    // Сортировка по PID, чтобы порядок был фиксированный
    //const sortedProcesses = [...processes].sort((a, b) => a.pid - b.pid);
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
 return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50">
      <motion.div
        initial={{ scale: 0.95, opacity: 0 }}
        animate={{ scale: 1, opacity: 1 }}
        exit={{ scale: 0.96, opacity: 0 }}
        className="bg-win-bg rounded-lg border border-win-border shadow-xl w-[90vw] max-w-4xl p-4 overflow-auto"
      >
        <div className="flex items-center justify-between mb-2">
          <div className="flex items-center gap-2">
            <span className="text-xl font-semibold text-win-text">Active Processes</span>
            <button
              onClick={() => setSearchOpen(v => !v)}
              className={`p-2 rounded-full hover:bg-win-control-hover transition-colors ${
                searchOpen ? "bg-win-control-active" : "bg-win-control"
              }`}
              title="Search"
            >
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="11" cy="11" r="8" />
                <line x1="21" y1="21" x2="16.65" y2="16.65" />
              </svg>
            </button>
            <AnimatePresence>
              {searchOpen && (
                <motion.input
                  key="search"
                  initial={{ width: 0, opacity: 0, marginLeft: 0 }}
                  animate={{ width: 220, opacity: 1, marginLeft: 8 }}
                  exit={{ width: 0, opacity: 0, marginLeft: 0 }}
                  transition={{ type: "spring", stiffness: 300, damping: 30 }}
                  autoFocus
                  type="text"
                  className="px-2 py-1 rounded bg-win-control border border-win-border text-win-text outline-none"
                  placeholder="Search by PID or Name"
                  value={search}
                  onChange={e => setSearch(e.target.value)}
                />
              )}
            </AnimatePresence>
          </div>
          <button
            className="px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700 transition-colors"
            onClick={onClose}
          >
            Close
          </button>
        </div>
        <div className="overflow-auto max-h-[60vh]">
          <table className="w-full border border-win-border bg-win-bg-secondary rounded text-win-text">
            <thead className="bg-win-header border-b border-win-border">
              <tr>
                <SortableTh
                  label="PID"
                  active={sortBy === 'pid'}
                  order={sortOrder}
                  onClick={() => handleSort('pid')}
                />
                <SortableTh
                  label="Name"
                  active={sortBy === 'name'}
                  order={sortOrder}
                  onClick={() => handleSort('name')}
                />
                <SortableTh
                  label="CPU"
                  active={sortBy === 'cpu_usage'}
                  order={sortOrder}
                  onClick={() => handleSort('cpu_usage')}
                />
                <SortableTh
                  label="Memory"
                  active={sortBy === 'memory'}
                  order={sortOrder}
                  onClick={() => handleSort('memory')}
                />
              </tr>
            </thead>
            <tbody>
              {filteredProcesses.length === 0 ? (
                <tr>
                  <td colSpan={4} className="py-4 text-center text-win-text-secondary">
                    No processes found.
                  </td>
                </tr>
              ) : (
                filteredProcesses.map(proc => (
                  <tr key={proc.pid} className="hover:bg-win-control-hover">
                    <td className="py-2 px-3">{proc.pid}</td>
                    <td className="py-2 px-3">{proc.name}</td>
                    <td className="py-2 px-3">{proc.cpu_usage.toFixed(1)}%</td>
                    <td className="py-2 px-3">{formatMemory(proc.memory)}</td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </motion.div>
    </div>
  )
}

// Вспомогательный компонент для заголовков с сортировкой
function SortableTh({ label, active, order, onClick }: SortableThProps) {
  return (
    <th onClick={onClick} className={`cursor-pointer px-4 py-2 text-left select-none`}>
      <span className={active ? "font-bold" : ""}>
        {label}
        {active && (
          <span className="ml-1">
            {order === 'asc' ? '▲' : order === 'desc' ? '▼' : ''}
          </span>
        )}
      </span>
    </th>
  );
}

// Функция форматирования памяти
function formatMemory(bytes: number): string {
  const units = ['B', 'KB', 'MB', 'GB']
  let value = bytes
  let unitIndex = 0
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024
    unitIndex++
  }
  return `${value.toFixed(unitIndex > 0 ? 1 : 0)} ${units[unitIndex]}`
}
