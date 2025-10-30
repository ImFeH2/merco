import { useEffect, useState } from 'react'
import { ChevronRight, ChevronDown, File, Folder, FolderOpen } from 'lucide-react'
import { api } from '@/services/api'
import { FileContextMenu } from '@/components/FileContextMenu'
import { CreateFileModal } from '@/components/CreateFileModal'
import type { FileNode, FileNodeType } from '@/types'

interface FileTreeProps {
  onFileSelect: (path: string, content: string) => void
  onRefresh?: () => void
}

interface TreeNodeProps {
  node: FileNode
  level: number
  onFileSelect: (path: string, content: string) => void
  onRefresh?: () => void
  onNewFile?: (parentPath: string) => void
}

interface ContextMenuState {
  x: number
  y: number
  node: FileNode
}

type ContextMenuStateOrNull = ContextMenuState | null

const TreeNode = ({ node, level, onFileSelect, onRefresh, onNewFile }: TreeNodeProps) => {
  const [isExpanded, setIsExpanded] = useState(false)
  const [children, setChildren] = useState<FileNode[]>([])
  const [isLoading, setIsLoading] = useState(false)
  const [contextMenu, setContextMenu] = useState<ContextMenuStateOrNull>(null)

  const isDirectory = node.type === 'directory'

  const loadChildren = async () => {
    setIsLoading(true)
    try {
      const response = await api.source.get({ path: node.path })
      if (response.type === 'directory') {
        setChildren(response.children)
      }
    } catch (error) {
      console.error('Failed to load directory:', error)
    } finally {
      setIsLoading(false)
    }
  }

  const handleClick = async () => {
    if (isDirectory) {
      if (!isExpanded && children.length === 0) {
        await loadChildren()
      }
      setIsExpanded(!isExpanded)
    } else {
      try {
        const response = await api.source.get({ path: node.path })
        if (response.type === 'file') {
          onFileSelect(response.path, response.content)
        }
      } catch (error) {
        console.error('Failed to load file:', error)
      }
    }
  }

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault()
    setContextMenu({
      x: e.clientX,
      y: e.clientY,
      node,
    })
  }

  const handleDelete = async () => {
    if (confirm(`Are you sure you want to delete ${node.name}?`)) {
      try {
        await api.source.delete({ path: node.path })
        onRefresh?.()
      } catch (error) {
        console.error('Failed to delete:', error)
      }
    }
  }

  const handleNewFile = () => {
    if (isDirectory) {
      onNewFile?.(node.path)
    }
  }

  const Icon = isDirectory
    ? isExpanded
      ? FolderOpen
      : Folder
    : File

  return (
    <div>
      <div
        className="flex items-center gap-1.5 px-2 py-1 cursor-pointer hover:bg-gray-50 rounded-md transition-colors"
        style={{ paddingLeft: `${level * 12 + 8}px` }}
        onClick={handleClick}
        onContextMenu={handleContextMenu}
      >
        {isDirectory && (
          <div className="w-4 h-4 flex items-center justify-center">
            {isLoading ? (
              <div className="w-3 h-3 border-2 border-gray-300 border-t-gray-600 rounded-full animate-spin" />
            ) : isExpanded ? (
              <ChevronDown className="w-4 h-4 text-gray-500" />
            ) : (
              <ChevronRight className="w-4 h-4 text-gray-500" />
            )}
          </div>
        )}
        {!isDirectory && <div className="w-4" />}
        <Icon className="w-4 h-4 text-gray-500 flex-shrink-0" />
        <span className="text-sm text-gray-700 truncate">{node.name}</span>
      </div>
      {isDirectory && isExpanded && (
        <div>
          {children.map((child) => (
            <TreeNode
              key={child.path}
              node={child}
              level={level + 1}
              onFileSelect={onFileSelect}
              onRefresh={async () => {
                await loadChildren()
                onRefresh?.()
              }}
              onNewFile={onNewFile}
            />
          ))}
        </div>
      )}
      {contextMenu && (
        <FileContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          isDirectory={isDirectory}
          onClose={() => setContextMenu(null)}
          onNewFile={handleNewFile}
          onDelete={handleDelete}
        />
      )}
    </div>
  )
}

export const FileTree = ({ onFileSelect, onRefresh }: FileTreeProps) => {
  const [rootChildren, setRootChildren] = useState<FileNode[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [createFileModal, setCreateFileModal] = useState<{ parentPath: string } | null>(null)

  const loadRoot = async () => {
    setIsLoading(true)
    try {
      const response = await api.source.get({ path: '' })
      if (response.type === 'directory') {
        setRootChildren(response.children)
      }
    } catch (err) {
      console.error('Failed to load root directory:', err)
      setError('Failed to load strategies')
    } finally {
      setIsLoading(false)
    }
  }

  useEffect(() => {
    loadRoot()
  }, [])

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="w-6 h-6 border-2 border-gray-300 border-t-gray-600 rounded-full animate-spin" />
      </div>
    )
  }

  if (error) {
    return (
      <div className="p-4 text-sm text-red-600">
        {error}
      </div>
    )
  }

  const handleNewFile = (parentPath: string) => {
    setCreateFileModal({ parentPath })
  }

  const handleCreateFile = async (filename: string) => {
    if (createFileModal) {
      const filePath = createFileModal.parentPath
        ? `${createFileModal.parentPath}/${filename}`
        : filename
      try {
        await api.source.save({ path: filePath }, '')
        await loadRoot()
        onRefresh?.()
      } catch (error) {
        console.error('Failed to create file:', error)
      }
    }
  }

  return (
    <>
      <div className="py-2">
        {rootChildren.map((child) => (
          <TreeNode
            key={child.path}
            node={child}
            level={0}
            onFileSelect={onFileSelect}
            onRefresh={loadRoot}
            onNewFile={handleNewFile}
          />
        ))}
      </div>
      {createFileModal && (
        <CreateFileModal
          isOpen={true}
          parentPath={createFileModal.parentPath}
          onClose={() => setCreateFileModal(null)}
          onConfirm={handleCreateFile}
        />
      )}
    </>
  )
}
