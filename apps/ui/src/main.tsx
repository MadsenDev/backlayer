import { Suspense, lazy } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
const App = lazy(() => import('./App.tsx'))

function BootScreen() {
  return (
    <main className="h-dvh overflow-hidden p-3">
      <div className="glass-panel flex h-full w-full items-center justify-center">
        <div className="surface-panel w-full max-w-[420px] p-8 text-center">
          <div className="mx-auto mb-5 flex size-14 items-center justify-center rounded-full border border-cyan-300/25 bg-cyan-300/10 text-cyan-200">
            <div className="size-5 animate-spin rounded-full border-2 border-current border-t-transparent" />
          </div>
          <div className="section-title">Backlayer</div>
          <div className="mt-2 text-xl font-semibold tracking-[-0.03em] text-white">
            Starting manager
          </div>
          <div className="mt-3 text-sm leading-6 text-slate-300">
            Loading the desktop UI and preparing the runtime connection.
          </div>
        </div>
      </div>
    </main>
  )
}

createRoot(document.getElementById('root')!).render(
  <Suspense fallback={<BootScreen />}>
    <App />
  </Suspense>,
)
