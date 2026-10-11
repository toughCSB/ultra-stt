import React from 'react'
import ReactDOM from 'react-dom/client'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import App from './App'
import { ErrorBoundary } from './components/ErrorBoundary'
import './i18n'
import './styles/globals.css'

// The capsule floats above other apps, so it must not paint the app canvas.
// Mark the root with the window label and let the stylesheet key off it.
try {
  const label = getCurrentWebviewWindow().label
  document.documentElement.dataset.window = label
  if (label !== 'capsule') {
    document.documentElement.classList.add('app-canvas')
  }
} catch {
  // Outside Tauri, for example in tests, fall back to the normal canvas.
  document.documentElement.classList.add('app-canvas')
}

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </React.StrictMode>,
)
