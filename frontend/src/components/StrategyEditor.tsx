import { useRef, useEffect } from 'react'
import { X } from 'lucide-react'
import Editor from '@monaco-editor/react'

interface EditorTab {
  path: string
  name: string
  displayName: string
  content: string
  isDirty: boolean
}

interface StrategyEditorProps {
  tabs: EditorTab[]
  activeTabIndex: number
  onTabChange: (index: number) => void
  onTabClose: (index: number) => void
  onContentChange: (index: number, content: string) => void
  onSave: (index: number) => void
}

export const StrategyEditor = ({
  tabs,
  activeTabIndex,
  onTabChange,
  onTabClose,
  onContentChange,
  onSave,
}: StrategyEditorProps) => {
  const editorRef = useRef<any>(null)
  const autoSaveTimerRef = useRef<NodeJS.Timeout | null>(null)

  const handleEditorDidMount = (editor: any, monaco: any) => {
    editorRef.current = editor
  }

  const activeTab = tabs[activeTabIndex]

  useEffect(() => {
    if (editorRef.current && activeTab) {
      editorRef.current.setValue(activeTab.content)
    }
  }, [activeTabIndex])

  useEffect(() => {
    if (activeTab?.isDirty) {
      if (autoSaveTimerRef.current) {
        clearTimeout(autoSaveTimerRef.current)
      }

      autoSaveTimerRef.current = setTimeout(() => {
        onSave(activeTabIndex)
      }, 2000)
    }

    return () => {
      if (autoSaveTimerRef.current) {
        clearTimeout(autoSaveTimerRef.current)
      }
    }
  }, [activeTab?.content, activeTab?.isDirty, activeTabIndex, onSave])

  const getLanguage = (filename: string) => {
    if (filename.endsWith('.rs')) return 'rust'
    if (filename.endsWith('.toml')) return 'toml'
    if (filename.endsWith('.json')) return 'json'
    if (filename.endsWith('.md')) return 'markdown'
    return 'plaintext'
  }

  if (tabs.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center bg-white">
        <div className="text-center">
          <p className="text-sm text-gray-500">No file open</p>
          <p className="text-xs text-gray-400 mt-1">Select a file from the tree to edit</p>
        </div>
      </div>
    )
  }

  return (
    <div className="flex-1 flex flex-col bg-white">
      <div className="flex items-center gap-px bg-gray-100 border-b border-gray-200">
        {tabs.map((tab, index) => (
          <div
            key={tab.path}
            className={`group flex items-center gap-2 px-4 py-2 border-r border-gray-200 cursor-pointer transition-colors ${index === activeTabIndex
              ? 'bg-white'
              : 'bg-gray-50 hover:bg-gray-100'
              }`}
            onClick={() => onTabChange(index)}
          >
            <span className="text-sm text-gray-700">
              {tab.displayName}
              {tab.isDirty && <span className="ml-1 text-gray-500">•</span>}
            </span>
            <button
              onClick={(e) => {
                e.stopPropagation()
                onTabClose(index)
              }}
              className="opacity-0 group-hover:opacity-100 transition-opacity"
            >
              <X className="w-3.5 h-3.5 text-gray-500 hover:text-gray-700" />
            </button>
          </div>
        ))}
      </div>

      <div className="flex-1 relative">
        {activeTab && (
          <Editor
            height="100%"
            language={getLanguage(activeTab.name)}
            value={activeTab.content}
            onChange={(value) => {
              if (value !== undefined) {
                onContentChange(activeTabIndex, value)
              }
            }}
            onMount={handleEditorDidMount}
            theme="vs"
            options={{
              fontSize: 14,
              lineNumbers: 'on',
              minimap: { enabled: false },
              scrollBeyondLastLine: false,
              wordWrap: 'on',
              automaticLayout: true,
              padding: { top: 16, bottom: 16 },
              renderLineHighlight: 'line',
              cursorBlinking: 'smooth',
              smoothScrolling: true,
              fontFamily: '"JetBrains Mono", Menlo, Monaco, "Courier New", monospace',
              tabSize: 2,
              insertSpaces: true,
            }}
          />
        )}
      </div>
    </div>
  )
}
