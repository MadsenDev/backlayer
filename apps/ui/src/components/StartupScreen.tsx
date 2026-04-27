import { motion } from 'framer-motion'
import { FiRefreshCw } from 'react-icons/fi'
import type { ActiveTheme } from '../hooks/useTheme'

export function StartupScreen({ activeTheme }: { activeTheme: ActiveTheme }) {
  return (
    <main className="h-dvh overflow-hidden p-3" data-theme={activeTheme}>
      <div className="glass-panel flex h-full w-full items-center justify-center">
        <motion.div
          className="surface-panel w-full max-w-[400px] p-8 text-center"
          style={{ borderRadius: 'var(--radius-panel)' }}
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.3, ease: 'easeOut' }}
        >
          <div
            className="mx-auto mb-5 flex size-12 items-center justify-center"
            style={{
              borderRadius: 'var(--radius-panel)',
              border: '1px solid var(--accent-soft)',
              background: 'var(--accent-soft)',
              color: 'var(--accent)',
            }}
          >
            <FiRefreshCw className="size-5 animate-spin" />
          </div>
          <div className="section-title">Backlayer</div>
          <div
            className="mt-2 text-xl font-semibold tracking-[-0.02em]"
            style={{ color: 'var(--text-primary)' }}
          >
            Starting manager
          </div>
          <div className="mt-2 text-sm leading-relaxed" style={{ color: 'var(--text-secondary)' }}>
            Connecting to daemon and loading wallpapers…
          </div>
        </motion.div>
      </div>
    </main>
  )
}
