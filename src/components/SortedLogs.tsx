import React, { useState, useMemo } from 'react';
import { FixedSizeList as List } from 'react-window';
import AutoSizer from 'react-virtualized-auto-sizer';

const normalizeLogLevel = (level: string) => {
  const levelUpper = level.toUpperCase();

  if (levelUpper.startsWith('WARN')) return 'WARNING';
  if (levelUpper.startsWith('TRACE')) return 'TRACE';
  if (levelUpper.startsWith('DEBUG')) return 'DEBUG';
  if (levelUpper.startsWith('ERROR')) return 'ERROR';
  if (levelUpper.startsWith('INFO')) return 'INFO';

  return levelUpper;
};

const getLogColor = (level: string) => {
  const levelUpper = level.toUpperCase();

  if (levelUpper === 'ERROR') return 'text-red-500';
  if (levelUpper === 'WARNING') return 'text-yellow-500';
  if (levelUpper === 'INFO') return 'text-green-500';
  if (levelUpper === 'DEBUG') return 'text-blue-400';
  if (levelUpper === 'TRACE') return 'text-purple-400';

  return 'text-gray-400';
};

export function SortedLogs({ logs }: { logs: LogEntry[] }) {
  const [sortField, setSortField] = useState('timestamp');
  const [sortDirection, setSortDirection] = useState('desc');
  const [filterLevel, setFilterLevel] = useState('all');
  const [searchTerm, setSearchTerm] = useState('');

  const filteredLogs = useMemo(() => {
    return logs.filter((log) => {
      const normalizedLevel = normalizeLogLevel(log.level);
      const searchMatch = !searchTerm ||
        log.message.toLowerCase().includes(searchTerm.toLowerCase()) ||
        log.timestamp.toLowerCase().includes(searchTerm.toLowerCase());

      if (filterLevel === 'all') return searchMatch;

      switch (filterLevel) {
        case 'INFO':
          return normalizedLevel === 'INFO' && searchMatch;
        case 'WARNING':
          return normalizedLevel === 'WARNING' && searchMatch;
        case 'ERROR':
          return normalizedLevel === 'ERROR' && searchMatch;
        case 'DEBUG':
          return normalizedLevel === 'DEBUG' && searchMatch;
        case 'TRACE':
          return normalizedLevel === 'TRACE' && searchMatch;
        default:
          return normalizedLevel === filterLevel && searchMatch;
      }
    });
  }, [logs, filterLevel, searchTerm]);

  const sortedLogs = useMemo(() => {
    return [...filteredLogs].sort((a, b) => {
      const factor = sortDirection === 'asc' ? 1 : -1;
      if (sortField === 'timestamp') {
        return factor * a.timestamp.localeCompare(b.timestamp);
      } else if (sortField === 'level') {
        return factor * normalizeLogLevel(a.level).localeCompare(normalizeLogLevel(b.level));
      } else {
        return factor * a.message.localeCompare(b.message);
      }
    });
  }, [filteredLogs, sortField, sortDirection]);

  const toggleSort = (field: string) => {
    if (sortField === field) {
      setSortDirection(sortDirection === 'asc' ? 'desc' : 'asc');
    } else {
      setSortField(field);
      setSortDirection('asc');
    }
  };
  const TableRow = ({ index, style }: { index: number; style: React.CSSProperties }) => {
    const log = sortedLogs[index];
    const normalizedLevel = normalizeLogLevel(log.level);
    
    return (
      <tr style={style} className="hover:bg-win-bg-hover">
        <td className="px-6 py-4 whitespace-nowrap text-sm text-win-text-secondary">
          {log.timestamp}
        </td>
        <td className="px-6 py-4 whitespace-nowrap">
          <span className={`px-2 inline-flex text-xs leading-5 font-semibold rounded-full ${getLogColor(normalizedLevel)}`}>
            {normalizedLevel}
          </span>
        </td>
        <td className="px-6 py-4 text-sm text-win-text">
          {log.message}
        </td>
      </tr>
    );
  };
    return (
      <div className="flex flex-col h-full">
        <div className="p-4 border-b border-win-border">
          <div className="flex flex-col sm:flex-row gap-3">
            <div className="flex-1">
              <input
                type="text"
                placeholder="Search logs..."
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
                className="w-full px-3 py-2 bg-win-control hover:bg-win-control-hover text-win-text border border-win-border rounded focus:outline-none focus:border-win-border-focus"
              />
            </div>
            <div>
              <select
                value={filterLevel}
                onChange={(e) => setFilterLevel(e.target.value)}
                className="w-full px-3 py-2 bg-win-control hover:bg-win-control-hover text-win-text border border-win-border rounded focus:outline-none focus:border-win-border-focus"
              >
                <option value="all">All Levels</option>
                <option value="INFO">Info</option>
                <option value="WARNING">Warning</option>
                <option value="ERROR">Error</option>
                <option value="DEBUG">Debug</option>
                <option value="TRACE">Trace</option>
              </select>
            </div>
          </div>
        </div>

        {/* Контейнер с фиксированной высотой и скроллом */}
        <div className="flex-1 overflow-auto bg-win-log font-mono max-h-[calc(100vh-250px)]">
          {sortedLogs.length > 0 ? (
            <div className="min-w-full">
              <table className="min-w-full">
                {/* <thead className="bg-win-bg-secondary border-b border-win-border sticky top-0 z-10">
                  <tr>
                    <th onClick={() => toggleSort('timestamp')} className="px-6 py-3 text-left text-xs font-medium text-win-text uppercase tracking-wider cursor-pointer hover:bg-win-bg-hover">
                      Timestamp {sortField === 'timestamp' && (sortDirection === 'asc' ? '↑' : '↓')}
                    </th>
                    <th onClick={() => toggleSort('level')} className="px-6 py-3 text-left text-xs font-medium text-win-text uppercase tracking-wider cursor-pointer hover:bg-win-bg-hover">
                      Level {sortField === 'level' && (sortDirection === 'asc' ? '↑' : '↓')}
                    </th>
                    <th onClick={() => toggleSort('message')} className="px-6 py-3 text-left text-xs font-medium text-win-text uppercase tracking-wider cursor-pointer hover:bg-win-bg-hover">
                      Message {sortField === 'message' && (sortDirection === 'asc' ? '↑' : '↓')}
                    </th>
                  </tr>
                </thead> */}
                {/* <tbody>
                  {sortedLogs.map((log, index) => {
                    const normalizedLevel = normalizeLogLevel(log.level);
                    return (
                      <tr key={index} className="hover:bg-win-bg-hover">
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-win-text-secondary">
                          {log.timestamp}
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap">
                          <span className={`px-2 inline-flex text-xs leading-5 font-semibold rounded-full ${getLogColor(normalizedLevel)}`}>
                            {normalizedLevel}
                          </span>
                        </td>
                        <td className="px-6 py-4 text-sm text-win-text">
                          {log.message}
                        </td>
                      </tr>
                    );
                  })}
                </tbody> */}
              </table>
               <div className="h-[calc(100vh-300px)]">
              <AutoSizer disableWidth>
                  {({ height }) => (
                    <List
                      height={height}
                      width="100%"
                      itemSize={60}
                      itemCount={sortedLogs.length}
                      overscanCount={10}
                    >
                      {TableRow}
                    </List>
                  )}
                </AutoSizer>
            </div>
            </div>
          ) : (
            <div className="p-4 text-center text-win-text-secondary h-full flex items-center justify-center">
            No logs available
          </div>
          )}
        </div>
      </div>
    );
  }

  interface LogEntry {
    timestamp: string;
    level: string;
    message: string;
  }