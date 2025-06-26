import React from 'react'
export function GameSelector({ selectedGame, onGameSelect }) {
  // Mock list of games
  const games = [
    'Counter-Strike 2',
    'Dota 2',
    'Apex Legends',
    'Valorant',
    'League of Legends',
  ]
  return (
    <div>
      <label className="block text-sm mb-1.5 text-win-text">Select Game</label>
      <div className="relative">
        <select
          value={selectedGame}
          onChange={(e) => onGameSelect(e.target.value)}
          className="block w-full rounded border bg-win-control hover:bg-win-control-hover border-win-border py-1.5 pl-3 pr-10 text-win-text focus:outline-none focus:border-win-border-focus"
        >
          <option value="">Select a game</option>
          {games.map((game) => (
            <option key={game} value={game}>
              {game}
            </option>
          ))}
        </select>
      </div>
    </div>
  )
}
