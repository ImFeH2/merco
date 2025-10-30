import { useState } from 'react'
import { Plus } from 'lucide-react'
import { FileTree } from '@/components/FileTree'
import { StrategyEditor } from '@/components/StrategyEditor'
import { AddStrategyModal } from '@/components/AddStrategyModal'
import { api } from '@/services/api'

interface EditorTab {
  path: string
  name: string
  displayName: string
  content: string
  isDirty: boolean
}

export default function Strategy() {
  const [tabs, setTabs] = useState<EditorTab[]>([])
  const [activeTabIndex, setActiveTabIndex] = useState(-1)
  const [isModalOpen, setIsModalOpen] = useState(false)
  const [refreshKey, setRefreshKey] = useState(0)

  const updateDisplayNames = (newTabs: EditorTab[]) => {
    const nameGroups = new Map<string, EditorTab[]>()

    newTabs.forEach(tab => {
      const tabs = nameGroups.get(tab.name) || []
      tabs.push(tab)
      nameGroups.set(tab.name, tabs)
    })

    newTabs.forEach(tab => {
      const sameNameTabs = nameGroups.get(tab.name) || []

      if (sameNameTabs.length === 1) {
        tab.displayName = tab.name
      } else {
        const pathParts = tab.path.split('/')
        let displayName = tab.name

        for (let i = pathParts.length - 2; i >= 0; i--) {
          const parentDir = pathParts[i]
          displayName = `${parentDir}/${displayName}`

          const isUnique = sameNameTabs.every(t =>
            t === tab || !t.path.includes(`${parentDir}/${tab.name}`)
          )

          if (isUnique) break
        }

        tab.displayName = displayName
      }
    })

    return newTabs
  }

  const handleFileSelect = (path: string, content: string) => {
    const existingIndex = tabs.findIndex((tab) => tab.path === path)

    if (existingIndex >= 0) {
      setActiveTabIndex(existingIndex)
    } else {
      const fileName = path.split('/').pop() || path
      const newTab: EditorTab = {
        path,
        name: fileName,
        displayName: fileName,
        content,
        isDirty: false,
      }
      const updatedTabs = updateDisplayNames([...tabs, newTab])
      setTabs(updatedTabs)
      setActiveTabIndex(updatedTabs.length - 1)
    }
  }

  const handleTabChange = (index: number) => {
    setActiveTabIndex(index)
  }

  const handleTabClose = (index: number) => {
    const newTabs = tabs.filter((_, i) => i !== index)
    const updatedTabs = updateDisplayNames(newTabs)
    setTabs(updatedTabs)

    if (activeTabIndex === index) {
      setActiveTabIndex(updatedTabs.length > 0 ? Math.min(index, updatedTabs.length - 1) : -1)
    } else if (activeTabIndex > index) {
      setActiveTabIndex(activeTabIndex - 1)
    }
  }

  const handleContentChange = (index: number, content: string) => {
    const newTabs = [...tabs]
    newTabs[index] = {
      ...newTabs[index],
      content,
      isDirty: true,
    }
    setTabs(newTabs)
  }

  const handleSave = async (index: number) => {
    const tab = tabs[index]
    try {
      await api.source.save({ path: tab.path }, tab.content)
      const newTabs = [...tabs]
      newTabs[index] = {
        ...newTabs[index],
        isDirty: false,
      }
      setTabs(newTabs)
    } catch (error) {
      console.error('Failed to save file:', error)
    }
  }

  const handleAddStrategy = async (name: string) => {
    try {
      await api.strategy.add({ name })
      setRefreshKey(prev => prev + 1)
    } catch (error) {
      console.error('Failed to add strategy:', error)
    }
  }

  return (
    <div className="flex h-full bg-gray-50">
      <div className="w-64 flex-shrink-0 bg-white border-r border-gray-200 overflow-y-auto">
        <div className="flex items-center justify-between px-4 py-3 border-b border-gray-200">
          <h2 className="text-sm font-medium text-gray-900">Strategies</h2>
          <button
            onClick={() => setIsModalOpen(true)}
            className="p-1.5 text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded-md transition-colors"
            title="Add Strategy"
          >
            <Plus className="w-4 h-4" />
          </button>
        </div>
        <FileTree key={refreshKey} onFileSelect={handleFileSelect} />
      </div>

      <StrategyEditor
        tabs={tabs}
        activeTabIndex={activeTabIndex}
        onTabChange={handleTabChange}
        onTabClose={handleTabClose}
        onContentChange={handleContentChange}
        onSave={handleSave}
      />

      <AddStrategyModal
        isOpen={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        onConfirm={handleAddStrategy}
      />
    </div>
  )
}
