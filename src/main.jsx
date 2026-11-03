import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './styles/global.css'
import './styles/components.css'
import { LanguageProvider } from './i18n/LanguageContext'
import ErrorBoundary from './components/ErrorBoundary'
import App from './App.jsx'
import { api } from './services/api'

// Mark React as mounted and inform backend
window.__reactMounted = true;
try {
  api.invoke('reactMounted', { timestamp: Date.now() }).catch(() => {});
} catch {}

createRoot(document.getElementById('root')).render(
  <StrictMode>
    <ErrorBoundary>
      <LanguageProvider>
        <App />
      </LanguageProvider>
    </ErrorBoundary>
  </StrictMode>,
)
