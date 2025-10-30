import { useEffect, useRef } from 'react'
import { FilePlus, Trash2 } from 'lucide-react'

interface FileContextMenuProps {
  x: number
  y: number
  isDirectory: boolean
  onClose: () => void
  onNewFile: () => void
  onDelete: () => void
}

export const FileContextMenu = ({
  x,
  y,
  isDirectory,
  onClose,
  onNewFile,
  onDelete,
}: FileContextMenuProps) => {
  const menuRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose()
      }
    }

    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose()
      }
    }

    document.addEventListener('mousedown', handleClickOutside)
    document.addEventListener('keydown', handleEscape)

    return () => {
      document.removeEventListener('mousedown', handleClickOutside)
      document.removeEventListener('keydown', handleEscape)
    }
  }, [onClose])

  return (
    <div
      ref={menuRef}
      className="fixed bg-white rounded-lg shadow-lg border border-gray-200 py-1 min-w-[160px] z-50"
      style={{ left: x, top: y }}
    >
      {isDirectory && (
        <button
          onClick={() => {
            onNewFile()
            onClose()
          }}
          className="w-full flex items-center gap-2 px-3 py-2 text-sm text-gray-700 hover:bg-gray-50 transition-colors"
        >
          <FilePlus className="w-4 h-4" />
          <span>New File</span>
        </button>
      )}
      <button
        onClick={() => {
          onDelete()
          onClose()
        }}
        className="w-full flex items-center gap-2 px-3 py-2 text-sm text-red-600 hover:bg-red-50 transition-colors"
      >
        <Trash2 className="w-4 h-4" />
        <span>Delete</span>
      </button>
    </div>
  )
}
