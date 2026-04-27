import { AnimatePresence, motion } from 'framer-motion'
import { useEffect, useRef, useState } from 'react'
import { FiActivity, FiDisc, FiRefreshCw, FiSliders, FiX } from 'react-icons/fi'
import { loadAssetPreviewDataUrl } from '../api'
import type {
  AssetMetadata,
  ImageFitMode,
  MonitorAssignment,
  MonitorInfo,
  RendererSession,
} from '../types'

function importRuntimeNote(kind: AssetMetadata['kind']) {
  switch (kind) {
    case 'scene':
      return 'Scene playback is not yet implemented for imported Workshop scenes.'
    case 'web':
      return 'Web wallpaper playback is not yet implemented.'
    case 'video':
      return 'Video wallpapers play through the FFmpeg-backed runner. libmpv and hardware decode are pending.'
    default:
      return 'Imported Workshop item.'
  }
}

function PreviewImage({ asset }: { asset: AssetMetadata }) {
  const [url, setUrl] = useState<string | null>(null)
  const previewPath = asset.preview_image ?? (asset.kind === 'image' ? asset.entrypoint : null)

  useEffect(() => {
    let cancelled = false
    if (!previewPath) return
    void loadAssetPreviewDataUrl(previewPath)
      .then((u) => { if (!cancelled) setUrl(u) })
      .catch(() => {})
    return () => { cancelled = true }
  }, [previewPath])

  const fallback = `data:image/svg+xml;utf8,${encodeURIComponent(
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 380 214">
      <rect width="380" height="214" fill="#ece7e0"/>
      <text x="190" y="115" font-family="sans-serif" font-size="14" fill="#7a746e" text-anchor="middle">${asset.kind.toUpperCase()}</text>
    </svg>`,
  )}`

  return (
    <div
      className="w-full bg-cover bg-center"
      style={{
        aspectRatio: '16/9',
        backgroundImage: `url("${url ?? fallback}")`,
        background: url ? undefined : 'var(--panel-muted)',
      }}
    />
  )
}

const FIT_OPTIONS: ImageFitMode[] = ['cover', 'contain', 'stretch', 'center']

export function DetailPanel({
  asset,
  monitor,
  assignment,
  runtimeSession,
  busy,
  workshopEnabled,
  snapshotSource,
  onAssign,
  onClose,
  onEditScene,
  onReimport,
  onRemove,
  onImageFitChange,
  onOpenTechDetails,
}: {
  asset: AssetMetadata | null
  monitor: MonitorInfo | null
  assignment: MonitorAssignment | null
  runtimeSession: RendererSession | null
  busy: string | null
  workshopEnabled: boolean
  snapshotSource: string | undefined
  onAssign: () => void
  onClose: () => void
  onEditScene: () => void
  onReimport: () => void
  onRemove: () => void
  onImageFitChange: (fit: ImageFitMode) => void
  onOpenTechDetails: () => void
}) {
  const panelRef = useRef<HTMLDivElement>(null)
  const isConnected = snapshotSource === 'tauri'
  const isAssigned = assignment?.wallpaper.id === asset?.id
  const assignBusy = busy === `assign:${monitor?.id}`
  const currentFit = assignment?.settings.image_fit ?? asset?.image_fit ?? 'cover'
  const imageSettingsBusy = busy === `assignment-settings:${monitor?.id}`

  const runtimeLabel = runtimeSession
    ? runtimeSession.status.kind === 'ready'
      ? `${runtimeSession.spec.backend}${runtimeSession.status.paused_reason ? ' · paused' : ' · live'}`
      : runtimeSession.status.reason
    : null

  // Close on Escape
  useEffect(() => {
    if (!asset) return
    const handleKey = (e: KeyboardEvent) => { if (e.key === 'Escape') onClose() }
    window.addEventListener('keydown', handleKey)
    return () => window.removeEventListener('keydown', handleKey)
  }, [asset, onClose])

  return (
    <AnimatePresence>
      {asset ? (
        <>
          {/* Backdrop (click to close) */}
          <motion.div
            className="absolute inset-0 z-10"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.18 }}
            style={{ background: 'rgba(28,22,18,0.18)' }}
            onClick={onClose}
          />

          {/* Panel */}
          <motion.div
            ref={panelRef}
            className="absolute inset-y-0 right-0 z-20 flex w-full flex-col overflow-hidden sm:w-[380px]"
            initial={{ x: 40, opacity: 0 }}
            animate={{ x: 0, opacity: 1 }}
            exit={{ x: 40, opacity: 0 }}
            transition={{ duration: 0.22, ease: 'easeOut' }}
            style={{
              background: 'var(--panel-bg)',
              borderLeft: '1px solid var(--border)',
              boxShadow: 'var(--shadow-chrome)',
            }}
            onClick={(e) => e.stopPropagation()}
          >
            {/* Header */}
            <div
              className="flex shrink-0 items-center justify-between gap-3 px-4 py-3"
              style={{ borderBottom: '1px solid var(--border)' }}
            >
              <div className="min-w-0">
                <div
                  className="truncate text-sm font-semibold"
                  style={{ color: 'var(--text-primary)' }}
                >
                  {asset.name}
                </div>
                <div className="mt-0.5 flex items-center gap-2">
                  <span
                    className="text-[10px] font-medium uppercase tracking-[0.16em]"
                    style={{ color: 'var(--accent)' }}
                  >
                    {asset.kind}
                  </span>
                  {isAssigned && (
                    <span
                      className="text-[10px] font-medium uppercase tracking-[0.16em]"
                      style={{ color: 'var(--text-tertiary)' }}
                    >
                      · active
                    </span>
                  )}
                </div>
              </div>
              <button
                type="button"
                className="icon-button shrink-0"
                style={{ borderRadius: 'var(--radius-control)' }}
                onClick={onClose}
              >
                <FiX className="size-4" />
              </button>
            </div>

            {/* Scrollable body */}
            <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain">
              {/* Preview */}
              <PreviewImage asset={asset} />

              <div className="space-y-3 p-4">
                {/* Assign button */}
                {monitor ? (
                  <button
                    type="button"
                    disabled={!isConnected || assignBusy}
                    onClick={onAssign}
                    className="primary-button w-full disabled:cursor-not-allowed disabled:opacity-60"
                    style={{ borderRadius: 'var(--radius-control)' }}
                  >
                    <FiDisc className="size-4" />
                    {assignBusy ? 'Assigning…' : `Set on ${monitor.output_name}`}
                  </button>
                ) : null}

                {/* Runtime status */}
                {runtimeLabel ? (
                  <div
                    className="flex items-center gap-2.5 px-3 py-2.5 text-sm"
                    style={{
                      borderRadius: 'var(--radius-control)',
                      border: '1px solid var(--border)',
                      background: 'var(--panel-muted)',
                      color: 'var(--text-secondary)',
                    }}
                  >
                    <FiActivity className="size-3.5 shrink-0" style={{ color: 'var(--accent)' }} />
                    {runtimeLabel}
                  </div>
                ) : null}

                {/* Image fit */}
                {asset.kind === 'image' && monitor ? (
                  <div
                    className="p-3"
                    style={{
                      borderRadius: 'var(--radius-panel)',
                      border: '1px solid var(--border)',
                      background: 'var(--panel-muted)',
                    }}
                  >
                    <div
                      className="mb-2.5 text-[11px] font-medium uppercase tracking-[0.18em]"
                      style={{ color: 'var(--text-tertiary)' }}
                    >
                      Image Fit
                    </div>
                    <div className="grid grid-cols-4 gap-1.5">
                      {FIT_OPTIONS.map((fit) => (
                        <button
                          key={fit}
                          type="button"
                          disabled={imageSettingsBusy}
                          onClick={() => onImageFitChange(fit)}
                          className="py-1.5 text-xs capitalize transition"
                          style={{
                            borderRadius: 'var(--radius-control)',
                            border: currentFit === fit ? '1px solid var(--accent)' : '1px solid var(--border)',
                            background: currentFit === fit ? 'var(--accent-soft)' : 'var(--panel-bg)',
                            color: currentFit === fit ? 'var(--accent)' : 'var(--text-secondary)',
                            fontWeight: currentFit === fit ? 600 : 400,
                          }}
                        >
                          {fit}
                        </button>
                      ))}
                    </div>
                  </div>
                ) : null}

                {/* Workshop import note */}
                {workshopEnabled && asset.source_kind === 'wallpaper_engine_import' ? (
                  <div
                    className="p-3 text-sm"
                    style={{
                      borderRadius: 'var(--radius-panel)',
                      border: '1px solid var(--border)',
                      background: 'var(--info-soft)',
                      color: 'var(--text-secondary)',
                    }}
                  >
                    <div
                      className="mb-1 text-xs font-semibold uppercase tracking-[0.16em]"
                      style={{ color: 'var(--text-primary)' }}
                    >
                      Workshop import
                    </div>
                    {importRuntimeNote(asset.kind)}
                  </div>
                ) : null}

                {/* Compatibility warnings */}
                {asset.compatibility.warnings.length > 0 ? (
                  <div
                    className="p-3 text-sm"
                    style={{
                      borderRadius: 'var(--radius-panel)',
                      border: '1px solid #c38a24',
                      background: 'var(--warning-soft)',
                      color: '#8f6418',
                    }}
                  >
                    <div className="mb-1 text-xs font-semibold uppercase tracking-[0.16em]">
                      Compatibility
                    </div>
                    <div className="space-y-1">
                      {asset.compatibility.warnings.slice(0, 3).map((w) => (
                        <div key={w}>{w}</div>
                      ))}
                    </div>
                  </div>
                ) : null}

                {/* Actions */}
                <div className="space-y-2">
                  {/* Tech details */}
                  <button
                    type="button"
                    onClick={onOpenTechDetails}
                    className="secondary-button w-full"
                    style={{ borderRadius: 'var(--radius-control)' }}
                  >
                    <FiActivity className="size-4" />
                    Technical Details
                  </button>

                  {/* Edit scene */}
                  {asset.kind === 'scene' && asset.source_kind === 'native' ? (
                    <button
                      type="button"
                      disabled={busy === `load-scene:${asset.id}` || !isConnected}
                      onClick={onEditScene}
                      className="secondary-button w-full disabled:cursor-not-allowed disabled:opacity-60"
                      style={{ borderRadius: 'var(--radius-control)' }}
                    >
                      <FiSliders className={busy === `load-scene:${asset.id}` ? 'size-4 animate-spin' : 'size-4'} />
                      {busy === `load-scene:${asset.id}` ? 'Opening…' : 'Edit scene'}
                    </button>
                  ) : null}

                  {/* Workshop reimport / remove */}
                  {workshopEnabled && asset.source_kind === 'wallpaper_engine_import' && asset.import_metadata?.source_path ? (
                    <div className="grid grid-cols-2 gap-2">
                      <button
                        type="button"
                        disabled={busy === `reimport:${asset.id}` || !isConnected}
                        onClick={onReimport}
                        className="secondary-button disabled:cursor-not-allowed disabled:opacity-60"
                        style={{ borderRadius: 'var(--radius-control)' }}
                      >
                        <FiRefreshCw className={busy === `reimport:${asset.id}` ? 'size-4 animate-spin' : 'size-4'} />
                        Reimport
                      </button>
                      <button
                        type="button"
                        disabled={busy === `remove:${asset.id}` || !isConnected}
                        onClick={onRemove}
                        className="disabled:cursor-not-allowed disabled:opacity-60 inline-flex items-center justify-center gap-2 px-4 py-2.5 text-sm font-medium transition"
                        style={{
                          borderRadius: 'var(--radius-control)',
                          border: '1px solid #b55056',
                          background: 'var(--danger-soft)',
                          color: '#9c3f4f',
                        }}
                      >
                        <FiX className="size-4" />
                        Remove
                      </button>
                    </div>
                  ) : null}
                </div>
              </div>
            </div>
          </motion.div>
        </>
      ) : null}
    </AnimatePresence>
  )
}
