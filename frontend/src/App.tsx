import { useState } from 'react'
import Navbar from '@/components/Navbar'
import MarketData from '@/pages/MarketData'
import Strategy from '@/pages/Strategy'
import Backtest from '@/pages/Backtest'
import Optimization from '@/pages/Optimization'
import Live from '@/pages/Live'
import Settings from '@/pages/Settings'

function App() {
  const [activeTab, setActiveTab] = useState('market')

  const renderPage = () => {
    switch (activeTab) {
      case 'market':
        return <MarketData />
      case 'strategy':
        return <Strategy />
      case 'backtest':
        return <Backtest />
      case 'optimization':
        return <Optimization />
      case 'live':
        return <Live />
      case 'settings':
        return <Settings />
      default:
        return <MarketData />
    }
  }

  return (
    <div className="flex flex-col h-screen bg-gray-50">
      <Navbar activeTab={activeTab} onTabChange={setActiveTab} />
      <main className="flex-1 overflow-hidden">{renderPage()}</main>
    </div>
  )
}

export default App
