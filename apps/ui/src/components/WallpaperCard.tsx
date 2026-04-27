import { memo, useEffect, useRef, useState } from 'react'
import { FiZap } from 'react-icons/fi'
import { loadAssetPreviewDataUrl } from '../api'
import type { AssetMetadata } from '../types'

function assetSummary(asset: AssetMetadata) {
  if (asset.kind === 'shader') {
    return asset.animated ? 'Animated WGSL shader' : 'Static WGSL shader'
  }
  if (asset.kind === 'image') {
    return `Image · ${asset.image_fit ?? 'cover'} fit`
  }
  if (asset.kind === 'scene') {
    return asset.source_kind === 'wallpaper_engine_import'
      ? 'Workshop scene · playback pending'
      : 'Native scene'
  }
  if (asset.kind === 'web') {
    return asset.source_kind === 'wallpaper_engine_import'
      ? 'Workshop web · playback pending'
      : 'Web wallpaper'
  }
  return asset.source_kind === 'wallpaper_engine_import'
    ? 'Workshop video'
    : 'Video · FFmpeg runner'
}

function assetPreviewSvg(asset: AssetMetadata) {
  switch (asset.id) {
    case 'demo.neon-grid':
      return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 160">
        <defs>
          <linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stop-color="#07111a"/>
            <stop offset="100%" stop-color="#0c3244"/>
          </linearGradient>
          <pattern id="grid" width="28" height="28" patternUnits="userSpaceOnUse">
            <path d="M 28 0 L 0 0 0 28" fill="none" stroke="#73e7d8" stroke-opacity="0.42" stroke-width="1"/>
          </pattern>
        </defs>
        <rect width="320" height="160" fill="url(#bg)"/>
        <rect width="320" height="160" fill="url(#grid)"/>
      </svg>`
    default: {
      const label = asset.kind.toUpperCase()
      return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 160">
        <rect width="320" height="160" fill="#ece7e0"/>
        <text x="160" y="88" font-family="sans-serif" font-size="13" fill="#7a746e" text-anchor="middle">${label}</text>
      </svg>`
    }
  }
}

function assetPreviewDataUri(asset: AssetMetadata) {
  return `data:image/svg+xml;utf8,${encodeURIComponent(assetPreviewSvg(asset))}`
}

const AssetPreview = memo(function AssetPreview({ asset }: { asset: AssetMetadata }) {
  const [previewUrl, setPreviewUrl] = useState<string | null>(null)
  const [visible, setVisible] = useState(false)
  const containerRef = useRef<HTMLDivElement | null>(null)
  const previewPath = asset.preview_image ?? (asset.kind === 'image' ? asset.entrypoint : null)

  useEffect(() => {
    const node = containerRef.current
    if (!node || typeof IntersectionObserver === 'undefined') {
      setVisible(true)
      return
    }
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          setVisible(true)
          observer.disconnect()
        }
      },
      { rootMargin: '240px' },
    )
    observer.observe(node)
    return () => observer.disconnect()
  }, [])

  useEffect(() => {
    let cancelled = false
    if (!visible || !previewPath) {
      setPreviewUrl(null)
      return
    }
    void loadAssetPreviewDataUrl(previewPath)
      .then((url) => { if (!cancelled) setPreviewUrl(url) })
      .catch(() => { if (!cancelled) setPreviewUrl(null) })
    return () => { cancelled = true }
  }, [previewPath, visible])

  return (
    <div
      ref={containerRef}
      className="h-full w-full bg-cover bg-center"
      style={{ backgroundImage: `url("${previewUrl ?? assetPreviewDataUri(asset)}")` }}
    />
  )
})

export const WallpaperCard = memo(function WallpaperCard({
  asset,
  selected,
  assigned,
  workshopEnabled,
  onContextMenu,
}: {
  asset: AssetMetadata
  selected: boolean
  assigned: boolean
  workshopEnabled: boolean
  onContextMenu?: (e: React.MouseEvent) => void
}) {
  const detailChips: React.ReactNode[] = []

  if (asset.kind === 'shader' && asset.animated) {
    detailChips.push(
      <span key="animated" className="control-chip">
        <FiZap className="size-3" /> animated
      </span>,
    )
  }
  if (asset.kind === 'image' && asset.image_fit) {
    detailChips.push(
      <span key="fit" className="control-chip">{asset.image_fit}</span>,
    )
  }
  if (workshopEnabled && asset.source_kind === 'wallpaper_engine_import') {
    detailChips.push(
      <span key="source" className="control-chip">workshop</span>,
    )
  }

  const compatColor = asset.compatibility.status === 'supported'
    ? 'var(--accent)'
    : asset.compatibility.status === 'partial'
      ? '#b8780e'
      : '#a84048'

  return (
    <div
      className="group grid h-full min-h-[260px] grid-rows-[152px_minmax(0,1fr)] overflow-hidden transition"
      style={{
        border: selected ? '2px solid var(--accent)' : '1px solid var(--border)',
        borderRadius: 'var(--radius-panel)',
        background: selected ? 'var(--accent-soft)' : 'var(--panel-bg)',
        boxShadow: selected ? 'var(--shadow-chrome)' : 'none',
      }}
      onContextMenu={onContextMenu}
    >
      {/* Thumbnail */}
      <div
        className="relative"
        style={{
          borderBottom: '1px solid var(--border)',
          background: 'var(--panel-muted)',
        }}
      >
        <AssetPreview asset={asset} />
        <div className="pointer-events-none absolute inset-x-0 bottom-0 h-16 bg-gradient-to-t from-black/20 to-transparent" />
        {/* Type badge */}
        <div className="absolute left-2.5 top-2.5">
          <span className="preview-badge">
            {asset.source_kind === 'wallpaper_engine_import' ? 'Workshop' : asset.kind}
          </span>
        </div>
        {/* Assigned indicator */}
        {assigned && (
          <div
            className="absolute right-2.5 top-2.5 flex items-center gap-1 px-2 py-1 text-[10px] font-medium uppercase tracking-[0.16em]"
            style={{
              borderRadius: 'var(--radius-control)',
              background: 'var(--accent)',
              color: 'var(--accent-contrast)',
            }}
          >
            Active
          </div>
        )}
      </div>

      {/* Info */}
      <div className="flex min-h-0 flex-col justify-between gap-3 p-3">
        <div>
          <div
            className="truncate text-sm font-semibold"
            style={{ color: 'var(--text-primary)' }}
          >
            {asset.name}
          </div>
          <div
            className="mt-0.5 truncate text-[11px]"
            style={{ color: 'var(--text-tertiary)' }}
          >
            {assetSummary(asset)}
          </div>
        </div>
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex flex-wrap gap-1.5">{detailChips}</div>
          <span
            className="text-[10px] font-medium uppercase tracking-[0.16em]"
            style={{ color: compatColor }}
          >
            {asset.compatibility.status}
          </span>
        </div>
      </div>
    </div>
  )
})
