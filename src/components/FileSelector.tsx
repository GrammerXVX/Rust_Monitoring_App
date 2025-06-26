import { open } from '@tauri-apps/plugin-dialog'

interface FileSelectorProps {
  onFileSelect: (path: string) => void
}

export function FileSelector({ onFileSelect }: FileSelectorProps) {
  const handleSelect = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'Log Files', extensions: ['log', 'txt'] }]
      })

      if (selected && !Array.isArray(selected)) {
        onFileSelect(selected)
      }
    } catch (error) {
      console.error('Error selecting file:', error)
    }
  }

  return (
    <button
      onClick={handleSelect}
      className="px-3 py-1.5 text-sm rounded border bg-win-control hover:bg-win-control-hover border-win-border focus:outline-none focus:border-win-border-focus"
    >
      Select Log File
    </button>
  )
}