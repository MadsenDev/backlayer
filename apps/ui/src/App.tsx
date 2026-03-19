import { motion } from 'framer-motion'
import { memo, startTransition, useDeferredValue, useEffect, useMemo, useRef, useState } from 'react'
import type { ChangeEvent, ReactNode } from 'react'
import { convertFileSrc } from '@tauri-apps/api/core'
import {
  FiActivity,
  FiAlertCircle,
  FiCpu,
  FiDisc,
  FiDownload,
  FiFilm,
  FiFilter,
  FiImage,
  FiMonitor,
  FiRefreshCw,
  FiSliders,
  FiTrash2,
  FiZap,
  FiX,
} from 'react-icons/fi'
import {
  assignWallpaper,
  createNativeAsset,
  createSceneAsset,
  fetchRuntimeSnapshot,
  importWorkshopPath,
  loadAssetPreviewDataUrl,
  loadEditableSceneAsset,
  removeAsset,
  reimportAsset,
  suggestWorkshopPaths,
  updateAssignmentSettings,
  updatePausePolicy,
} from './api'
import type {
  AssetMetadata,
  AssignmentSettings,
  CreateNativeAssetRequest,
  CreateSceneAssetRequest,
  CreateSceneImageSourceRequest,
  EditableSceneImage,
  ImageFitMode,
  PausePolicy,
  RuntimeSnapshot,
  SceneBehaviorKind,
  SceneColorStop,
  SceneCurvePoint,
  SceneEmitterNode,
  SceneEmitterShape,
  SceneEffectKind,
  SceneEmitterPreset,
  SceneNode,
  SceneNormalizedRect,
  SceneNormalizedPoint,
  SceneParticleAreaNode,
  SceneSpriteNode,
} from './types'

type ThemePreference = 'system' | 'light' | 'dark'
type ActiveTheme = 'light' | 'dark'
type UINotice = {
  title: string
  detail: string
}
type ComposerUploadImage = {
  key: string
  name: string
  filename: string
  sourceUrl: string
  dataUrl?: string | null
  path?: string | null
  width?: number | null
  height?: number | null
}
type EditableScalarPoint = {
  id: string
  x: number
  y: number
}
type EditableColorStop = {
  id: string
  x: number
  color_hex: string
}
type ComposerLeftTab = 'layers' | 'assets' | 'add'
type ComposerViewportTool = 'select' | 'move' | 'scale' | 'rotate' | 'region' | 'polygon'
type AssetContextMenuState = {
  assetId: string
  x: number
  y: number
}
type CreateAssetKind = 'image' | 'scene' | 'shader' | 'video'

const THEME_PREFERENCE_KEY = 'backlayer.theme.preference'
const SCENE_EFFECT_OPTIONS: Array<{
  effect: SceneEffectKind
  label: string
  description: string
}> = [
  { effect: 'glow', label: 'Glow', description: 'Adds a pulsing bloom-like center haze.' },
  { effect: 'vignette', label: 'Vignette', description: 'Darkens the edges for depth and focus.' },
  { effect: 'scanlines', label: 'Scanlines', description: 'Adds a drifting line pass over the scene.' },
  { effect: 'fog', label: 'Fog', description: 'Adds a slow atmospheric haze pass for depth.' },
]
const SCENE_EMITTER_OPTIONS: Array<{
  preset: SceneEmitterPreset
  label: string
  description: string
}> = [
  { preset: 'embers', label: 'Embers', description: 'Warm floating sparks with upward drift.' },
  { preset: 'rain', label: 'Rain', description: 'Fast diagonal rainfall streaks.' },
  { preset: 'dust', label: 'Dust', description: 'Soft atmospheric particles for ambient depth.' },
  { preset: 'snow', label: 'Snow', description: 'Slow cold flakes drifting across the scene.' },
]

function defaultEmitterOriginX(preset: SceneEmitterPreset) {
  switch (preset) {
    case 'rain':
      return 0.55
    case 'snow':
      return 0.5
    case 'dust':
      return 0.5
    case 'embers':
      return 0.5
  }
}

function defaultEmitterOriginY(preset: SceneEmitterPreset) {
  switch (preset) {
    case 'rain':
      return 0
    case 'snow':
      return 0
    case 'dust':
      return 0.56
    case 'embers':
      return 1
  }
}

function defaultEmitterDirectionDeg(preset: SceneEmitterPreset) {
  switch (preset) {
    case 'embers':
      return -90
    case 'rain':
      return 100
    case 'dust':
      return -26
    case 'snow':
      return 92
  }
}

function defaultEmitterColorHex(preset: SceneEmitterPreset) {
  switch (preset) {
    case 'embers':
      return '#ff9452'
    case 'rain':
      return '#bedcff'
    case 'dust':
      return '#e0ecff'
    case 'snow':
      return '#f4f7ff'
  }
}

function defaultEmitterShape(preset: SceneEmitterPreset): SceneEmitterShape {
  switch (preset) {
    case 'rain':
      return 'line'
    case 'snow':
      return 'box'
    case 'dust':
      return 'box'
    case 'embers':
      return 'circle'
  }
}

function defaultEmitterRegionWidth(preset: SceneEmitterPreset) {
  switch (preset) {
    case 'rain':
      return 0.42
    case 'snow':
      return 0.28
    case 'dust':
      return 0.16
    case 'embers':
      return 0.14
  }
}

function defaultEmitterRegionHeight(preset: SceneEmitterPreset) {
  switch (preset) {
    case 'rain':
      return 0.08
    case 'snow':
      return 0.06
    case 'dust':
      return 0.12
    case 'embers':
      return 0.08
  }
}

function defaultEmitterRegionRadius(preset: SceneEmitterPreset) {
  switch (preset) {
    case 'rain':
      return 0.12
    case 'snow':
      return 0.18
    case 'dust':
      return 0.1
    case 'embers':
      return 0.09
  }
}

function defaultEmitterLineLength(preset: SceneEmitterPreset) {
  switch (preset) {
    case 'rain':
      return 0.42
    case 'snow':
      return 0.2
    case 'dust':
      return 0.16
    case 'embers':
      return 0.12
  }
}

function defaultEmitterMinSpeed(preset: SceneEmitterPreset) {
  switch (preset) {
    case 'embers':
      return 48
    case 'rain':
      return 320
    case 'dust':
      return 14
    case 'snow':
      return 20
  }
}

function defaultEmitterMaxSpeed(preset: SceneEmitterPreset) {
  switch (preset) {
    case 'embers':
      return 110
    case 'rain':
      return 620
    case 'dust':
      return 42
    case 'snow':
      return 58
  }
}

function defaultEmitterMinLife(preset: SceneEmitterPreset) {
  switch (preset) {
    case 'embers':
      return 2.8
    case 'rain':
      return 1.7
    case 'dust':
      return 4.5
    case 'snow':
      return 6
  }
}

function defaultEmitterMaxLife(preset: SceneEmitterPreset) {
  switch (preset) {
    case 'embers':
      return 5
    case 'rain':
      return 2.5
    case 'dust':
      return 9
    case 'snow':
      return 9
  }
}

function defaultEmitterSizeCurve(preset: SceneEmitterPreset): SceneCurvePoint[] {
  switch (preset) {
    case 'rain':
      return [{ x: 0, y: 0.7 }, { x: 1, y: 1 }]
    case 'snow':
      return [{ x: 0, y: 0.8 }, { x: 0.5, y: 1 }, { x: 1, y: 0.85 }]
    case 'dust':
      return [{ x: 0, y: 0.55 }, { x: 0.5, y: 1 }, { x: 1, y: 1.2 }]
    case 'embers':
      return [{ x: 0, y: 0.7 }, { x: 0.55, y: 1 }, { x: 1, y: 0.35 }]
  }
}

function defaultEmitterAlphaCurve(preset: SceneEmitterPreset): SceneCurvePoint[] {
  switch (preset) {
    case 'rain':
      return [{ x: 0, y: 0.9 }, { x: 1, y: 0.3 }]
    case 'snow':
      return [{ x: 0, y: 0.25 }, { x: 0.18, y: 0.7 }, { x: 1, y: 0.1 }]
    case 'dust':
      return [{ x: 0, y: 0.1 }, { x: 0.35, y: 0.55 }, { x: 1, y: 0 }]
    case 'embers':
      return [{ x: 0, y: 0.25 }, { x: 0.2, y: 1 }, { x: 1, y: 0 }]
  }
}

function defaultEmitterColorCurve(preset: SceneEmitterPreset): SceneColorStop[] {
  switch (preset) {
    case 'rain':
      return [{ x: 0, color_hex: '#e1f1ff' }, { x: 1, color_hex: '#7bb7ff' }]
    case 'snow':
      return [{ x: 0, color_hex: '#ffffff' }, { x: 1, color_hex: '#dbe8ff' }]
    case 'dust':
      return [{ x: 0, color_hex: '#fff1d9' }, { x: 1, color_hex: '#d5b98e' }]
    case 'embers':
      return [{ x: 0, color_hex: '#fff1af' }, { x: 0.55, color_hex: '#ff8b4a' }, { x: 1, color_hex: '#72250b' }]
  }
}

function defaultEffectColorHex(effect: SceneEffectKind) {
  switch (effect) {
    case 'glow':
      return '#ffc785'
    case 'vignette':
      return '#070d14'
    case 'scanlines':
      return '#ffd69b'
    case 'fog':
      return '#dbe8ff'
  }
}

function resolveEmitterOriginX(node: SceneEmitterNode) {
  return clamp01(node.origin_x ?? defaultEmitterOriginX(node.preset))
}

function resolveEmitterOriginY(node: SceneEmitterNode) {
  return clamp01(node.origin_y ?? defaultEmitterOriginY(node.preset))
}

function resolveEmitterDirectionDeg(node: SceneEmitterNode) {
  return node.direction_deg ?? defaultEmitterDirectionDeg(node.preset)
}

function resolveEmitterShape(node: SceneEmitterNode) {
  return node.shape ?? defaultEmitterShape(node.preset)
}

function resolveEmitterRegionWidth(node: SceneEmitterNode) {
  return clamp01(node.region_width ?? defaultEmitterRegionWidth(node.preset))
}

function resolveEmitterRegionHeight(node: SceneEmitterNode) {
  return clamp01(node.region_height ?? defaultEmitterRegionHeight(node.preset))
}

function resolveEmitterRegionRadius(node: SceneEmitterNode) {
  return Math.min(1, Math.max(0.01, node.region_radius ?? defaultEmitterRegionRadius(node.preset)))
}

function resolveEmitterLineLength(node: SceneEmitterNode) {
  return Math.min(1, Math.max(0.01, node.line_length ?? defaultEmitterLineLength(node.preset)))
}

function resolveEmitterLineAngleDeg(node: SceneEmitterNode) {
  return node.line_angle_deg ?? resolveEmitterDirectionDeg(node)
}

function resolveEmitterMinSpeed(node: SceneEmitterNode) {
  return Math.max(0, node.min_speed ?? defaultEmitterMinSpeed(node.preset))
}

function resolveEmitterMaxSpeed(node: SceneEmitterNode) {
  return Math.max(resolveEmitterMinSpeed(node), node.max_speed ?? defaultEmitterMaxSpeed(node.preset))
}

function resolveEmitterMinLife(node: SceneEmitterNode) {
  return Math.max(0.2, node.min_life ?? defaultEmitterMinLife(node.preset))
}

function resolveEmitterMaxLife(node: SceneEmitterNode) {
  return Math.max(resolveEmitterMinLife(node), node.max_life ?? defaultEmitterMaxLife(node.preset))
}

function resolveEmitterColorHex(node: SceneEmitterNode) {
  const value = node.color_hex?.trim()
  if (!value) {
    return defaultEmitterColorHex(node.preset)
  }
  const normalized = value.startsWith('#') ? value : `#${value}`
  return /^#[0-9a-fA-F]{6}$/.test(normalized) ? normalized.toLowerCase() : defaultEmitterColorHex(node.preset)
}

function resolveEmitterParticleImageKey(node: SceneEmitterNode) {
  const value = node.particle_image_key?.trim()
  return value ? value : null
}

function resolveEmitterParticleRotationDeg(node: SceneEmitterNode) {
  return clampDegrees(node.particle_rotation_deg ?? 0)
}

function resolveRenderedParticleAngleRad(node: SceneEmitterNode, vx: number, vy: number) {
  const offsetRad = (resolveEmitterParticleRotationDeg(node) * Math.PI) / 180
  if (node.preset === 'rain') {
    return Math.atan2(vy, vx) - (Math.PI / 2) + offsetRad
  }
  return offsetRad
}

function resolveScalarCurve(points: SceneCurvePoint[] | undefined, fallback: SceneCurvePoint[]) {
  const next = (points && points.length > 0 ? points : fallback).map((point) => ({
    x: clamp01(point.x),
    y: Math.min(2.5, Math.max(0, point.y)),
  })).sort((left, right) => left.x - right.x)
  if (next[0]?.x !== 0) {
    next.unshift({ x: 0, y: next[0]?.y ?? 1 })
  }
  if (next[next.length - 1]?.x !== 1) {
    next.push({ x: 1, y: next[next.length - 1]?.y ?? 1 })
  }
  return next
}

function resolveColorCurve(points: SceneColorStop[] | undefined, fallback: SceneColorStop[]) {
  const next = (points && points.length > 0 ? points : fallback).map((point) => ({
    x: clamp01(point.x),
    color_hex: normalizeColorHex(point.color_hex) ?? '#ffffff',
  })).sort((left, right) => left.x - right.x)
  if (next[0]?.x !== 0) {
    next.unshift({ x: 0, color_hex: next[0]?.color_hex ?? '#ffffff' })
  }
  if (next[next.length - 1]?.x !== 1) {
    next.push({ x: 1, color_hex: next[next.length - 1]?.color_hex ?? '#ffffff' })
  }
  return next
}

function resolveEffectColorHex(node: Extract<SceneNode, { kind: 'effect' }>) {
  const value = node.color_hex?.trim()
  if (!value) {
    return defaultEffectColorHex(node.effect)
  }
  const normalized = value.startsWith('#') ? value : `#${value}`
  return /^#[0-9a-fA-F]{6}$/.test(normalized) ? normalized.toLowerCase() : defaultEffectColorHex(node.effect)
}

function clamp01(value: number) {
  return Math.min(1, Math.max(0, value))
}

function clampDegrees(value: number) {
  if (!Number.isFinite(value)) {
    return 0
  }
  let normalized = value % 360
  if (normalized > 180) {
    normalized -= 360
  }
  if (normalized < -180) {
    normalized += 360
  }
  return normalized
}

function parseColorHex(colorHex: string) {
  const normalized = colorHex.replace('#', '')
  const parse = (start: number) => Number.parseInt(normalized.slice(start, start + 2), 16)
  const red = parse(0)
  const green = parse(2)
  const blue = parse(4)
  return {
    red: Number.isFinite(red) ? red : 255,
    green: Number.isFinite(green) ? green : 255,
    blue: Number.isFinite(blue) ? blue : 255,
  }
}

function normalizeColorHex(colorHex: string | null | undefined) {
  const value = colorHex?.trim()
  if (!value) {
    return null
  }
  const normalized = value.startsWith('#') ? value : `#${value}`
  return /^#[0-9a-fA-F]{6}$/.test(normalized) ? normalized.toLowerCase() : null
}

function App() {
  const [snapshot, setSnapshot] = useState<RuntimeSnapshot | null>(null)
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState<string | null>(null)
  const [uiError, setUiError] = useState<string | null>(null)
  const [selectedMonitorId, setSelectedMonitorId] = useState<string | null>(null)
  const [selectedAssetId, setSelectedAssetId] = useState<string | null>(null)
  const [settingsOpen, setSettingsOpen] = useState(false)
  const [monitorPickerOpen, setMonitorPickerOpen] = useState(false)
  const [advancedOpen, setAdvancedOpen] = useState(false)
  const [importOpen, setImportOpen] = useState(false)
  const [createName, setCreateName] = useState('')
  const [importPath, setImportPath] = useState('')
  const [suggestedWorkshopPaths, setSuggestedWorkshopPaths] = useState<string[]>([])
  const [importResult, setImportResult] = useState<AssetMetadata[]>([])
  const [assetSearch, setAssetSearch] = useState('')
  const [filtersOpen, setFiltersOpen] = useState(false)
  const [createPickerOpen, setCreatePickerOpen] = useState(false)
  const [sourceFilter, setSourceFilter] = useState<'all' | 'native' | 'wallpaper_engine_import'>('all')
  const [kindFilter, setKindFilter] = useState<'all' | 'image' | 'video' | 'shader' | 'scene' | 'web'>('all')
  const [compatibilityFilter, setCompatibilityFilter] = useState<'all' | 'supported' | 'partial' | 'unsupported'>('all')
  const [composerOpen, setComposerOpen] = useState(false)
  const [composerName, setComposerName] = useState('')
  const [composerBaseAssetId, setComposerBaseAssetId] = useState<string | null>(null)
  const [composerUploadImage, setComposerUploadImage] = useState<ComposerUploadImage | null>(null)
  const [composerExtraImages, setComposerExtraImages] = useState<ComposerUploadImage[]>([])
  const [composerEditingAssetId, setComposerEditingAssetId] = useState<string | null>(null)
  const [composerLoadingScene, setComposerLoadingScene] = useState(false)
  const [composerNodes, setComposerNodes] = useState<SceneNode[]>(() => buildDefaultComposerNodes())
  const [selectedComposerNodeId, setSelectedComposerNodeId] = useState<string | null>(null)
  const [composerLeftTab, setComposerLeftTab] = useState<ComposerLeftTab>('layers')
  const [composerViewportTool, setComposerViewportTool] = useState<ComposerViewportTool>('select')
  const [particleEditorNodeId, setParticleEditorNodeId] = useState<string | null>(null)
  const [notice, setNotice] = useState<UINotice | null>(null)
  const [assetContextMenu, setAssetContextMenu] = useState<AssetContextMenuState | null>(null)
  const [themePreference, setThemePreference] = useState<ThemePreference>(() => readThemePreference())
  const [systemTheme, setSystemTheme] = useState<ActiveTheme>(() => detectSystemTheme())
  const [documentVisible, setDocumentVisible] = useState(() => typeof document === 'undefined' ? true : !document.hidden)
  const [disconnectedPollMs, setDisconnectedPollMs] = useState(5000)
  const composerFileInputRef = useRef<HTMLInputElement | null>(null)
  const composerSpriteFileInputRef = useRef<HTMLInputElement | null>(null)
  const createImageFileInputRef = useRef<HTMLInputElement | null>(null)
  const createVideoFileInputRef = useRef<HTMLInputElement | null>(null)
  const createShaderFileInputRef = useRef<HTMLInputElement | null>(null)

  async function refreshSnapshot(includeAssets = false) {
    const nextSnapshot = await fetchRuntimeSnapshot({
      includeAssets,
    })
    startTransition(() => {
      setSnapshot((current) => ({
        ...nextSnapshot,
        assets: includeAssets ? nextSnapshot.assets : current?.assets ?? nextSnapshot.assets,
      }))
      setLoading(false)
    })
    if (nextSnapshot.source === 'tauri_disconnected') {
      setDisconnectedPollMs((current) => Math.min(current * 2, 30000))
    } else {
      setDisconnectedPollMs(5000)
    }
    return nextSnapshot
  }

  useEffect(() => {
    let cancelled = false

    void fetchRuntimeSnapshot({ includeAssets: true }).then((nextSnapshot) => {
      if (cancelled) {
        return
      }

      startTransition(() => {
        setSnapshot(nextSnapshot)
        setLoading(false)
      })
    })

    return () => {
      cancelled = true
    }
  }, [])

  useEffect(() => {
    const updateVisibility = () => {
      setDocumentVisible(!document.hidden)
    }
    document.addEventListener('visibilitychange', updateVisibility)
    window.addEventListener('focus', updateVisibility)
    window.addEventListener('blur', updateVisibility)
    return () => {
      document.removeEventListener('visibilitychange', updateVisibility)
      window.removeEventListener('focus', updateVisibility)
      window.removeEventListener('blur', updateVisibility)
    }
  }, [])

  useEffect(() => {
    if (snapshot?.source !== 'tauri' && snapshot?.source !== 'tauri_disconnected') {
      return
    }

    const intervalMs = snapshot.source === 'tauri_disconnected'
      ? disconnectedPollMs
      : documentVisible ? 5000 : 30000
    const timeout = window.setTimeout(() => {
      void refreshSnapshot(false).catch(() => {})
    }, intervalMs)

    return () => {
      window.clearTimeout(timeout)
    }
  }, [disconnectedPollMs, documentVisible, snapshot?.source])

  const assignments = snapshot?.state.assignments ?? []
  const monitors = snapshot?.state.monitors ?? []
  const assets = snapshot?.assets ?? []
  const workshopEnabled = snapshot?.featureFlags.workshop_enabled ?? false
  const deferredAssetSearch = useDeferredValue(assetSearch)
  const runtimeSessions = snapshot?.state.runtime.sessions ?? []
  const unresolvedAssignments = snapshot?.state.runtime.unresolved_assignments ?? []
  const recentEvents = snapshot?.state.recent_events ?? []
  const hasActiveFilters =
    sourceFilter !== 'all' || kindFilter !== 'all' || compatibilityFilter !== 'all'
  const activeFilterSummary = [
    sourceFilter !== 'all' ? sourceFilter === 'wallpaper_engine_import' ? 'Workshop' : 'Native' : null,
    kindFilter !== 'all' ? kindFilter : null,
    compatibilityFilter !== 'all' ? compatibilityFilter : null,
  ].filter(Boolean) as string[]
  const activeTheme: ActiveTheme = themePreference === 'system' ? systemTheme : themePreference
  const filteredAssets = useMemo(() => assets.filter((asset) => {
    const search = deferredAssetSearch.trim().toLowerCase()
    const matchesSearch =
      search.length === 0
      || asset.name.toLowerCase().includes(search)
      || asset.id.toLowerCase().includes(search)
      || asset.kind.toLowerCase().includes(search)
      || asset.compatibility.status.toLowerCase().includes(search)
    const matchesSource = sourceFilter === 'all' || asset.source_kind === sourceFilter
    const matchesKind = kindFilter === 'all' || asset.kind === kindFilter
    const matchesCompatibility =
      compatibilityFilter === 'all' || asset.compatibility.status === compatibilityFilter

    return matchesSearch && matchesSource && matchesKind && matchesCompatibility
  }), [assets, compatibilityFilter, deferredAssetSearch, kindFilter, sourceFilter])
  const selectedMonitor = useMemo(() =>
    monitors.find((monitor) => monitor.id === selectedMonitorId)
    ?? monitors.find((monitor) => monitor.focused)
    ?? monitors[0]
    ?? null
  , [monitors, selectedMonitorId])
  const selectedAssignment = useMemo(() => selectedMonitor
    ? assignments.find(
        ({ monitor_id }) =>
          monitor_id === selectedMonitor.id || monitor_id === selectedMonitor.output_name,
      ) ?? null
    : null
  , [assignments, selectedMonitor])
  const selectedAsset = useMemo(() =>
    assets.find((asset) => asset.id === selectedAssetId)
    ?? selectedAssignment?.wallpaper
    ?? assets[0]
    ?? null
  , [assets, selectedAssetId, selectedAssignment])
  const composerBaseAsset = composerBaseAssetId
    ? assets.find((asset) => asset.id === composerBaseAssetId) ?? null
    : null
  const composerSourceName = composerUploadImage?.name ?? composerBaseAsset?.name ?? null
  const selectedComposerNode = useMemo(() =>
    composerNodes.find((node) => node.id === selectedComposerNodeId) ?? composerNodes[0] ?? null
  , [composerNodes, selectedComposerNodeId])
  const particleEditorNodeCandidate = composerNodes.find((node) => node.id === particleEditorNodeId) ?? null
  const particleEditorNode = particleEditorNodeCandidate?.kind === 'emitter' ? particleEditorNodeCandidate : null
  const selectedRuntime = selectedMonitor
    ? runtimeSessions.find((session) => session.spec.monitor_id === selectedMonitor.id) ?? null
    : null
  const contextMenuAsset = assetContextMenu
    ? assets.find((asset) => asset.id === assetContextMenu.assetId) ?? null
    : null

  useEffect(() => {
    if (!selectedMonitor && monitors[0]) {
      setSelectedMonitorId(monitors[0].id)
    }
  }, [monitors, selectedMonitor])

  useEffect(() => {
    if (!selectedAsset && assets[0]) {
      setSelectedAssetId(assets[0].id)
    }
  }, [assets, selectedAsset])

  useEffect(() => {
    if (composerOpen && composerSourceName && !composerName) {
      setComposerName(`${composerSourceName} Scene`)
    }
  }, [composerName, composerOpen, composerSourceName])

  useEffect(() => {
    if (!composerOpen) {
      return
    }

    if (!selectedComposerNodeId && composerNodes[0]) {
      setSelectedComposerNodeId(composerNodes[0].id)
      return
    }

    if (selectedComposerNodeId && !composerNodes.some((node) => node.id === selectedComposerNodeId)) {
      setSelectedComposerNodeId(composerNodes[0]?.id ?? null)
    }
  }, [composerNodes, composerOpen, selectedComposerNodeId])

  useEffect(() => {
    if (!selectedComposerNode) {
      setComposerViewportTool('select')
      return
    }
    const allowedTools = resolveViewportToolsForNode(selectedComposerNode)
    if (!allowedTools.includes(composerViewportTool)) {
      setComposerViewportTool('select')
    }
  }, [composerViewportTool, selectedComposerNode])

  useEffect(() => {
    if (!notice) {
      return
    }

    const timeout = window.setTimeout(() => {
      setNotice(null)
    }, 4200)

    return () => {
      window.clearTimeout(timeout)
    }
  }, [notice])

  useEffect(() => {
    if (!assetContextMenu) {
      return
    }

    const closeMenu = () => setAssetContextMenu(null)
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setAssetContextMenu(null)
      }
    }

    window.addEventListener('pointerdown', closeMenu)
    window.addEventListener('keydown', handleKeyDown)
    return () => {
      window.removeEventListener('pointerdown', closeMenu)
      window.removeEventListener('keydown', handleKeyDown)
    }
  }, [assetContextMenu])

  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)')
    const updateTheme = (event?: MediaQueryListEvent) => {
      setSystemTheme(event?.matches ?? media.matches ? 'dark' : 'light')
    }

    updateTheme()
    media.addEventListener('change', updateTheme)
    return () => {
      media.removeEventListener('change', updateTheme)
    }
  }, [])

  useEffect(() => {
    window.localStorage.setItem(THEME_PREFERENCE_KEY, themePreference)
  }, [themePreference])

  useEffect(() => {
    document.documentElement.dataset.theme = activeTheme
    document.documentElement.style.colorScheme = activeTheme

    return () => {
      delete document.documentElement.dataset.theme
      document.documentElement.style.removeProperty('color-scheme')
    }
  }, [activeTheme])

  useEffect(() => {
    if (workshopEnabled) {
      return
    }

    if (sourceFilter === 'wallpaper_engine_import') {
      setSourceFilter('all')
    }
    if (importOpen) {
      setImportOpen(false)
    }
  }, [importOpen, sourceFilter, workshopEnabled])

  useEffect(() => {
    if (!importOpen) {
      setImportResult([])
      return
    }

    if (!workshopEnabled) {
      return
    }

    void suggestWorkshopPaths().then((paths) => {
      setSuggestedWorkshopPaths(paths)
      if (!importPath && paths[0]) {
        setImportPath(paths[0])
      }
    })
  }, [importOpen, importPath, workshopEnabled])

  async function handleAssignWallpaper(monitorId: string, assetId: string) {
    setBusy(`assign:${monitorId}`)
    setUiError(null)
    try {
      await assignWallpaper(monitorId, assetId)
      await refreshSnapshot(false)
    } catch (error) {
      setUiError(error instanceof Error ? error.message : String(error))
    } finally {
      setBusy(null)
    }
  }

  async function handlePausePolicyChange(nextPause: PausePolicy) {
    setBusy('pause')
    setUiError(null)
    try {
      await updatePausePolicy(nextPause)
      await refreshSnapshot(false)
    } catch (error) {
      setUiError(error instanceof Error ? error.message : String(error))
    } finally {
      setBusy(null)
    }
  }

  async function handleAssignmentSettingsChange(
    monitorId: string,
    settings: AssignmentSettings,
  ) {
    setBusy(`assignment-settings:${monitorId}`)
    setUiError(null)
    try {
      await updateAssignmentSettings(monitorId, settings)
      await refreshSnapshot(false)
    } catch (error) {
      setUiError(error instanceof Error ? error.message : String(error))
    } finally {
      setBusy(null)
    }
  }

  async function handleRefresh() {
    setBusy('refresh')
    setUiError(null)
    try {
      await refreshSnapshot(true)
    } catch (error) {
      setUiError(error instanceof Error ? error.message : String(error))
    } finally {
      setBusy(null)
    }
  }

  async function handleImportWorkshopPath() {
    if (!importPath.trim()) {
      setUiError('Enter a Wallpaper Engine workshop item path or workshop directory path.')
      return
    }

    setBusy('import')
    setUiError(null)
    try {
      const importedAssets = await importWorkshopPath(importPath.trim())
      setImportResult(importedAssets)
      if (importedAssets[0]) {
        setSelectedAssetId(importedAssets[0].id)
        setSourceFilter('wallpaper_engine_import')
        setKindFilter('all')
        setCompatibilityFilter('all')
        setAssetSearch('')
      }
      await refreshSnapshot(true)
    } catch (error) {
      setUiError(error instanceof Error ? error.message : String(error))
    } finally {
      setBusy(null)
    }
  }

  async function handleReimportAsset(assetId: string) {
    setBusy(`reimport:${assetId}`)
    setUiError(null)
    try {
      await reimportAsset(assetId)
      await refreshSnapshot(true)
    } catch (error) {
      setUiError(error instanceof Error ? error.message : String(error))
    } finally {
      setBusy(null)
    }
  }

  async function handleRemoveAsset(assetId: string) {
    setBusy(`remove:${assetId}`)
    setUiError(null)
    try {
      await removeAsset(assetId)
      if (selectedAssetId === assetId) {
        setSelectedAssetId(null)
      }
      await refreshSnapshot(true)
    } catch (error) {
      setUiError(error instanceof Error ? error.message : String(error))
    } finally {
      setBusy(null)
    }
  }

  async function handleCreateScene() {
    if (!composerBaseAsset && !composerUploadImage) {
      setUiError('Choose an image asset or pick a local image file before creating a scene.')
      return
    }

    const request: CreateSceneAssetRequest = {
      name: composerName.trim() || `${composerSourceName ?? 'Scene'} Scene`,
      existing_asset_id: composerEditingAssetId,
      base_asset_id: composerUploadImage?.dataUrl ? null : composerBaseAsset?.id ?? null,
      base_image_data_url: composerUploadImage?.dataUrl ?? null,
      base_image_filename: composerUploadImage?.filename ?? null,
      base_image_path: composerUploadImage?.dataUrl ? null : composerUploadImage?.path ?? null,
      extra_images: composerExtraImages.map((image) => ({
        key: image.key,
        filename: image.filename,
        data_url: image.dataUrl ?? null,
        existing_path: image.dataUrl ? null : image.path ?? null,
      })) satisfies CreateSceneImageSourceRequest[],
      nodes: composerNodes,
    }

    setBusy('create-scene')
    setUiError(null)
    try {
      const asset = await createSceneAsset(request)
      await refreshSnapshot(true)
      setSelectedAssetId(asset.id)
      setKindFilter('all')
      setComposerOpen(false)
      setComposerName('')
      setComposerBaseAssetId(null)
      setComposerUploadImage(null)
      setComposerExtraImages([])
      setComposerEditingAssetId(null)
      setComposerNodes(buildDefaultComposerNodes())
      setSelectedComposerNodeId(null)
      setComposerLeftTab('layers')
      setNotice({
        title: composerEditingAssetId ? 'Scene updated' : 'Scene created',
        detail: composerEditingAssetId
          ? `${asset.name} was updated in your library.`
          : `${asset.name} is now available in the browser as a native scene asset.`,
      })
    } catch (error) {
      setUiError(error instanceof Error ? error.message : String(error))
    } finally {
      setBusy(null)
    }
  }

  function openComposer(
    initialAsset: AssetMetadata | null = selectedAsset,
    preferredName?: string,
  ) {
    setUiError(null)
    setComposerLoadingScene(false)
    setComposerOpen(true)
    setComposerNodes(buildDefaultComposerNodes())
    setSelectedComposerNodeId('sprite-base')
    setComposerUploadImage(null)
    setComposerExtraImages([])
    setComposerEditingAssetId(null)
    setComposerLeftTab('layers')
    if (initialAsset?.kind === 'image') {
      setSelectedAssetId(initialAsset.id)
      setComposerBaseAssetId(initialAsset.id)
      setComposerName(preferredName?.trim() || `${initialAsset.name} Scene`)
    } else {
      setComposerBaseAssetId(null)
      setComposerName(preferredName?.trim() || '')
    }
    setCreateName('')
  }

function closeComposer() {
    setComposerOpen(false)
    setComposerLoadingScene(false)
    setComposerName('')
    setComposerBaseAssetId(null)
    setComposerUploadImage(null)
    setComposerExtraImages([])
    setComposerEditingAssetId(null)
    setComposerNodes(buildDefaultComposerNodes())
    setSelectedComposerNodeId(null)
    setComposerLeftTab('layers')
  }

  async function openComposerForExistingScene(asset: AssetMetadata) {
    setUiError(null)
    setComposerOpen(true)
    setComposerLoadingScene(true)
    setComposerEditingAssetId(asset.id)
    setComposerBaseAssetId(null)
    setComposerUploadImage(null)
    setComposerExtraImages([])
    setComposerNodes(buildDefaultComposerNodes())
    setSelectedComposerNodeId(null)
    setComposerLeftTab('layers')
    setBusy(`load-scene:${asset.id}`)
    try {
      const editable = await loadEditableSceneAsset(asset.id)
      const baseImage = editable.images.find((image) => image.key === 'base')
      if (!baseImage) {
        throw new Error('Editable scene is missing its base image.')
      }

      const toComposerImage = (image: EditableSceneImage): ComposerUploadImage => ({
        key: image.key,
        name: image.filename.replace(/\.[^.]+$/, '') || image.key,
        filename: image.filename,
        sourceUrl: convertFileSrc(image.path),
        path: image.path,
        width: image.width ?? null,
        height: image.height ?? null,
      })

      setSelectedAssetId(asset.id)
      setComposerEditingAssetId(asset.id)
      setComposerBaseAssetId(null)
      setComposerUploadImage(toComposerImage(baseImage))
      setComposerExtraImages(editable.images.filter((image) => image.key !== 'base').map(toComposerImage))
      setComposerNodes(editable.document.nodes)
      setSelectedComposerNodeId(editable.document.nodes[0]?.id ?? null)
      setComposerName(editable.asset.name)
      setComposerLeftTab('layers')
    } catch (error) {
      setComposerOpen(false)
      setUiError(error instanceof Error ? error.message : String(error))
    } finally {
      setComposerLoadingScene(false)
      setBusy(null)
    }
  }

  function handleUseSelectedImage() {
    if (!selectedAsset || selectedAsset.kind !== 'image') {
      setUiError('Select an image wallpaper in the browser first, or choose a local image file.')
      return
    }

    setComposerUploadImage(null)
    setComposerExtraImages([])
    setComposerEditingAssetId(null)
    setComposerBaseAssetId(selectedAsset.id)
    setComposerName(`${selectedAsset.name} Scene`)
    setComposerNodes(buildDefaultComposerNodes())
    setSelectedComposerNodeId('sprite-base')
    setComposerLeftTab('assets')
  }

  async function readFileAsDataUrl(file: File, errorMessage: string) {
    return await new Promise<string>((resolve, reject) => {
      const reader = new FileReader()
      reader.onload = () => resolve(String(reader.result ?? ''))
      reader.onerror = () => reject(new Error(errorMessage))
      reader.readAsDataURL(file)
    })
  }

  async function handleCreateNativeAssetFilePicked(
    event: ChangeEvent<HTMLInputElement>,
    kind: Extract<CreateAssetKind, 'image' | 'video' | 'shader'>,
  ) {
    const file = event.currentTarget.files?.[0]
    if (!file) {
      return
    }

    setBusy(`create-native:${kind}`)
    setUiError(null)
    try {
      const dataUrl = await readFileAsDataUrl(file, `Failed to read ${kind} file.`)
      if (!dataUrl) {
        throw new Error(`Failed to read ${kind} file.`)
      }

      const request: CreateNativeAssetRequest = {
        name: createName.trim() || file.name.replace(/\.[^.]+$/, '') || `Imported ${kind}`,
        kind,
        data_url: dataUrl,
        filename: file.name,
      }
      const asset = await createNativeAsset(request)
      await refreshSnapshot(true)
      setSelectedAssetId(asset.id)
      setCreatePickerOpen(false)
      setCreateName('')
      setNotice({
        title: `${asset.name} created`,
        detail: `${asset.name} is now available in the wallpaper browser as a native ${asset.kind} asset.`,
      })
    } catch (error) {
      setUiError(error instanceof Error ? error.message : String(error))
    } finally {
      setBusy(null)
      event.currentTarget.value = ''
    }
  }

  async function handleComposerFilePicked(event: ChangeEvent<HTMLInputElement>) {
    const file = event.currentTarget.files?.[0]
    if (!file) {
      return
    }

    const dataUrl = await readFileAsDataUrl(file, 'Failed to read image file.').catch((error) => {
      setUiError(error instanceof Error ? error.message : String(error))
      return ''
    })

    if (!dataUrl) {
      return
    }

    const baseName = file.name.replace(/\.[^.]+$/, '') || 'Imported Image'
    setComposerBaseAssetId(null)
    setComposerEditingAssetId(null)
    setComposerUploadImage({
      key: 'base',
      name: baseName,
      filename: file.name,
      sourceUrl: dataUrl,
      dataUrl,
    })
    setComposerExtraImages([])
    setComposerName(`${baseName} Scene`)
    setComposerNodes(buildDefaultComposerNodes())
    setSelectedComposerNodeId('sprite-base')
    setComposerLeftTab('assets')
    event.currentTarget.value = ''
  }

  async function handleComposerSpriteFilePicked(event: ChangeEvent<HTMLInputElement>) {
    const file = event.currentTarget.files?.[0]
    if (!file) {
      return
    }

    const dataUrl = await readFileAsDataUrl(file, 'Failed to read sprite image file.').catch((error) => {
      setUiError(error instanceof Error ? error.message : String(error))
      return ''
    })

    if (!dataUrl) {
      return
    }

    const baseName = file.name.replace(/\.[^.]+$/, '') || 'Overlay Image'
    const keyBase = slugifyComposerKey(baseName)
    const key = uniqueComposerImageKey(keyBase, composerExtraImages)
    const nextImage = {
      key,
      name: baseName,
      filename: file.name,
      sourceUrl: dataUrl,
      dataUrl,
    }

    setComposerExtraImages((items) => [...items, nextImage])
    const node = createComposerSpriteNode(key, baseName)
    setComposerNodes((items) => [...items, node])
    setSelectedComposerNodeId(node.id)
    setComposerLeftTab('layers')
    event.currentTarget.value = ''
  }

  if (loading && !snapshot) {
    return <StartupScreen activeTheme={activeTheme} />
  }

  return (
    <main
      className="h-dvh overflow-hidden p-3"
      data-theme={activeTheme}
      onContextMenu={(event) => event.preventDefault()}
    >
      <motion.div
        className="glass-panel flex h-full w-full flex-col overflow-hidden"
        initial={{ opacity: 0, y: 18 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.45, ease: 'easeOut' }}
      >
        <header className="flex flex-col gap-3 border-b border-white/10 bg-white/[0.03] px-4 py-3 lg:px-5">
          <div className="flex items-center justify-between gap-4">
            <div className="flex min-w-0 items-center gap-3">
              <div className="surface-muted p-2 text-cyan-200">
                <FiDisc className="size-4" />
              </div>
              <div className="min-w-0">
                <div className="section-title">Backlayer</div>
                <div className="mt-0.5 flex min-w-0 items-center gap-3">
                  <h1 className="truncate text-base font-semibold tracking-[-0.03em] text-white md:text-lg">
                    Wallpaper Browser
                  </h1>
                </div>
              </div>
            </div>

            <div className="flex shrink-0 items-center gap-2">
              <StatusPill loading={loading} source={snapshot?.source} />
              <button
                className="toolbar-button"
                onClick={() => setMonitorPickerOpen(true)}
                type="button"
              >
                <FiMonitor className="size-4" />
                {selectedMonitor?.output_name ?? 'Select monitor'}
              </button>
              <button
                className="toolbar-button"
                onClick={() => setCreatePickerOpen(true)}
                type="button"
              >
                <FiImage className="size-4" />
                Create
              </button>
              <button
                className="toolbar-button"
                onClick={() => setSettingsOpen(true)}
                type="button"
              >
                <FiSliders className="size-4" />
                Settings
              </button>
              {workshopEnabled ? (
                <button
                  className="toolbar-button"
                  onClick={() => setImportOpen(true)}
                  type="button"
                >
                  <FiDownload className="size-4" />
                  Import
                </button>
              ) : null}
              <button
                className="toolbar-button disabled:opacity-60"
                disabled={busy === 'refresh'}
                onClick={() => {
                  void handleRefresh()
                }}
                type="button"
              >
                <FiRefreshCw className={busy === 'refresh' ? 'size-4 animate-spin' : 'size-4'} />
                {busy === 'refresh' ? 'Refreshing' : 'Refresh'}
              </button>
            </div>
          </div>

          <div className="surface-panel flex min-w-0 flex-col gap-2 px-3.5 py-3 lg:flex-row lg:items-center lg:justify-between">
            <div className="min-w-0">
              <div className="flex min-w-0 flex-wrap items-center gap-2">
                <div className="truncate text-base font-semibold text-white">
                  {selectedAsset?.name ?? 'Select a wallpaper'}
                </div>
                <span className="control-chip">{selectedMonitor?.output_name ?? 'No monitor'}</span>
                <span className="control-chip">
                  {selectedAssignment?.wallpaper.id === selectedAsset?.id ? 'assigned' : 'preview target'}
                </span>
                {selectedAsset ? <span className="control-chip">{selectedAsset.kind}</span> : null}
                {selectedAsset?.kind === 'shader' && selectedAsset.animated ? <span className="control-chip"><FiZap className="size-3" /> animated</span> : null}
                {selectedAsset?.kind === 'image' && selectedAsset.image_fit ? <span className="control-chip">{selectedAsset.image_fit}</span> : null}
                {workshopEnabled && selectedAsset?.source_kind === 'wallpaper_engine_import' ? <span className="control-chip">workshop import</span> : null}
                {selectedAsset ? <span className="control-chip">{selectedAsset.compatibility.status}</span> : null}
              </div>
              <p className="mt-1 line-clamp-1 max-w-3xl text-sm leading-5 text-slate-400">
                {selectedAsset
                  ? assetSummary(selectedAsset)
                  : 'Pick a wallpaper from the browser to inspect it and apply it to the selected monitor.'}
              </p>
            </div>
            {selectedMonitor ? (
              <button
                type="button"
                disabled={!selectedAsset || snapshot?.source !== 'tauri' || busy === `assign:${selectedMonitor.id}`}
                onClick={() => {
                  if (!selectedAsset) {
                    return
                  }
                  void handleAssignWallpaper(selectedMonitor.id, selectedAsset.id)
                }}
                className="primary-button shrink-0 disabled:cursor-not-allowed disabled:opacity-60"
              >
                <FiDisc className="size-4" />
                {busy === `assign:${selectedMonitor.id}` ? 'Assigning...' : `Apply to ${selectedMonitor.output_name}`}
              </button>
            ) : null}
          </div>
          {notice ? (
            <div className="tone-info flex items-start justify-between gap-3 px-3.5 py-3 text-sm">
              <div>
                <div className="font-medium">{notice.title}</div>
                <div className="mt-1 text-cyan-50/90">{notice.detail}</div>
              </div>
              <button className="icon-button shrink-0" onClick={() => setNotice(null)} type="button">
                <FiX className="size-4" />
              </button>
            </div>
          ) : null}
        </header>

        <div className="min-h-0 flex-1 bg-white/5">
          <section className="grid h-full min-h-0 grid-rows-[minmax(0,1fr)] gap-5 overflow-hidden bg-slate-950/55 p-4 lg:p-6">
            <div className="grid min-h-0 gap-5 xl:grid-cols-[minmax(0,1fr)_340px]">
                <div className="surface-panel grid min-h-0 h-full grid-rows-[auto_auto_minmax(0,1fr)] overflow-hidden p-4">
                  <div className="mb-3 shrink-0 flex flex-col gap-3 border-b border-white/10 pb-3 xl:flex-row xl:items-center xl:justify-between">
                    <div className="flex min-w-0 items-center gap-3">
                      <p className="section-title">Wallpaper Browser</p>
                      <span className="control-chip shrink-0">{filteredAssets.length} / {assets.length} assets</span>
                    </div>
                    <div className="flex w-full flex-col gap-2 xl:w-auto xl:flex-row xl:items-center">
                      <input
                        className="field-shell w-full xl:w-80"
                        onChange={(event) => setAssetSearch(event.currentTarget.value)}
                        placeholder="Search wallpapers..."
                        type="text"
                        value={assetSearch}
                      />
                      <div className="flex items-center gap-2">
                        <button
                          type="button"
                          onClick={() => setFiltersOpen((open) => !open)}
                          className={[
                            'inline-flex shrink-0 items-center gap-2 border px-3.5 py-2 text-sm transition',
                            filtersOpen || hasActiveFilters
                              ? 'border-emerald-300/30 bg-emerald-300/10 text-emerald-100'
                              : 'border-white/10 bg-white/5 text-white hover:bg-white/10',
                          ].join(' ')}
                        >
                          <FiFilter className="size-4" />
                          Filters
                          {hasActiveFilters ? <span className="control-chip border-emerald-300/20 text-emerald-100">{activeFilterSummary.length}</span> : null}
                        </button>
                        {hasActiveFilters ? (
                          <button
                            type="button"
                            onClick={() => {
                              setSourceFilter('all')
                              setKindFilter('all')
                              setCompatibilityFilter('all')
                            }}
                            className="secondary-button shrink-0"
                          >
                            Clear
                          </button>
                        ) : null}
                      </div>
                    </div>
                  </div>

                  <div className="mb-3 shrink-0 space-y-2">
                    {hasActiveFilters ? (
                      <div className="flex flex-wrap gap-2">
                        {activeFilterSummary.map((filter) => (
                          <span key={filter} className="control-chip">
                            {filter}
                          </span>
                        ))}
                      </div>
                    ) : null}

                    {filtersOpen ? (
                      <div className="surface-muted p-3">
                        <div className="space-y-3">
                          <div>
                            <div className="mb-2 text-[11px] uppercase tracking-[0.16em] text-slate-500">Source</div>
                            <div className="flex flex-wrap gap-2">
                              <FilterChip
                                active={sourceFilter === 'all'}
                                label="All Sources"
                                onClick={() => setSourceFilter('all')}
                              />
                              <FilterChip
                                active={sourceFilter === 'native'}
                                label="Native"
                                onClick={() => setSourceFilter('native')}
                              />
                              {workshopEnabled ? (
                                <FilterChip
                                  active={sourceFilter === 'wallpaper_engine_import'}
                                  label="Workshop"
                                  onClick={() => setSourceFilter('wallpaper_engine_import')}
                                />
                              ) : null}
                            </div>
                          </div>
                          <div>
                            <div className="mb-2 text-[11px] uppercase tracking-[0.16em] text-slate-500">Type</div>
                            <div className="flex flex-wrap gap-2">
                              {(['all', 'image', 'video', 'shader', 'scene', 'web'] as const).map((kind) => (
                                <FilterChip
                                  key={kind}
                                  active={kindFilter === kind}
                                  label={kind === 'all' ? 'All Types' : kind}
                                  onClick={() => setKindFilter(kind)}
                                />
                              ))}
                            </div>
                          </div>
                          <div>
                            <div className="mb-2 text-[11px] uppercase tracking-[0.16em] text-slate-500">State</div>
                            <div className="flex flex-wrap gap-2">
                              {(['all', 'supported', 'partial', 'unsupported'] as const).map((status) => (
                                <FilterChip
                                  key={status}
                                  active={compatibilityFilter === status}
                                  label={status === 'all' ? 'All States' : status}
                                  onClick={() => setCompatibilityFilter(status)}
                                />
                              ))}
                            </div>
                          </div>
                        </div>
                      </div>
                    ) : null}
                  </div>

                  <div className="min-h-0 overflow-y-auto overflow-x-hidden overscroll-contain pr-1 pb-2">
                    <div className="grid gap-4 pb-1 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4">
                      {filteredAssets.map((asset, index) => {
                        const selected = selectedAsset?.id === asset.id
                        return (
                          <motion.button
                            key={asset.id}
                            type="button"
                            onClick={() => setSelectedAssetId(asset.id)}
                            onContextMenu={(event) => {
                              event.preventDefault()
                              event.stopPropagation()
                              setAssetContextMenu({
                                assetId: asset.id,
                                x: event.clientX,
                                y: event.clientY,
                              })
                            }}
                            initial={{ opacity: 0, y: 12 }}
                            animate={{ opacity: 1, y: 0 }}
                            transition={{ duration: 0.22, delay: index * 0.015 }}
                            className="text-left"
                          >
                            <AssetCard asset={asset} selected={selected} workshopEnabled={workshopEnabled} />
                          </motion.button>
                        )
                      })}
                    </div>
                    {filteredAssets.length === 0 ? (
                      <div className="mt-4">
                        <EmptyState text="No wallpapers match the current search and filter combination." />
                      </div>
                    ) : null}
                  </div>
                </div>

                <div className="surface-panel grid min-h-0 h-full grid-rows-[auto_minmax(0,1fr)] overflow-hidden p-4">
                  <div className="mb-4 flex shrink-0 items-center gap-3">
                    <div className="rounded-2xl border border-white/10 bg-white/5 p-2 text-slate-200">
                      <FiCpu className="size-4" />
                    </div>
                    <div>
                      <p className="section-title">Inspector</p>
                      <div className="mt-1 text-sm text-slate-300">Wallpaper, assignment, and runtime details in one pane.</div>
                    </div>
                  </div>

                  <div className="min-h-0 overflow-y-auto overflow-x-hidden overscroll-contain pr-1 pb-2">
                    <div className="space-y-4">
                    <InspectorHero
                      asset={selectedAsset}
                      monitorName={selectedMonitor?.output_name ?? 'No monitor'}
                      runtimeLabel={selectedRuntime
                        ? selectedRuntime.status.kind === 'ready'
                          ? `${selectedRuntime.spec.backend} • ${selectedRuntime.status.persistent ? 'live worker' : 'snapshot'}${selectedRuntime.status.paused_reason ? ` • paused` : ''}`
                          : selectedRuntime.status.reason
                        : 'No runtime session'}
                    />
                    {selectedAsset?.kind === 'image' && selectedMonitor ? (
                      <ImageFitControl
                        busy={busy === `assignment-settings:${selectedMonitor.id}`}
                        currentFit={selectedAssignment?.settings.image_fit ?? selectedAsset.image_fit ?? 'cover'}
                        onChange={(imageFit) => {
                          void handleAssignmentSettingsChange(selectedMonitor.id, { image_fit: imageFit })
                        }}
                      />
                    ) : null}
                    <div className="grid gap-3">
                      <InspectorMiniCard
                        icon={<FiMonitor className="size-4" />}
                        title="Assignment"
                        emphasis={selectedAssignment?.wallpaper.name ?? 'Unassigned'}
                        meta={[
                          selectedMonitor
                            ? `${selectedMonitor.width}x${selectedMonitor.height} @ ${selectedMonitor.refresh_rate.toFixed(2)}Hz`
                            : 'No monitor selected',
                          selectedMonitor ? `Position ${selectedMonitor.x}, ${selectedMonitor.y}` : null,
                        ]}
                      />
                      <InspectorMiniCard
                        icon={<FiCpu className="size-4" />}
                        title="Runtime"
                        emphasis={selectedRuntime?.status.kind === 'ready'
                          ? `${selectedRuntime.spec.backend} • ${selectedRuntime.status.persistent ? 'live worker' : 'snapshot'}`
                          : selectedRuntime?.status.reason ?? 'No runtime session'}
                        meta={[
                          selectedRuntime?.status.kind === 'ready'
                            ? `Output ${selectedRuntime.status.output_name}`
                            : null,
                          selectedRuntime?.status.kind === 'ready' && selectedRuntime.status.paused_reason
                            ? `Paused: ${selectedRuntime.status.paused_reason}`
                            : null,
                          selectedRuntime?.status.kind === 'ready'
                            ? selectedRuntime.status.detail ?? null
                            : null,
                        ]}
                      />
                    </div>
                    {workshopEnabled && selectedAsset?.source_kind === 'wallpaper_engine_import' ? (
                      <div className="rounded-[22px] border border-cyan-300/20 bg-cyan-300/10 p-4 text-sm text-cyan-100">
                        <div className="mb-2 font-medium">Imported Runtime Mode</div>
                        <div className="leading-6 text-cyan-50/90">
                          {importRuntimeNote(selectedAsset.kind)}
                        </div>
                      </div>
                    ) : null}
                    <button
                      type="button"
                      onClick={() => setAdvancedOpen(true)}
                      className="flex w-full items-center gap-3 rounded-[22px] border border-white/10 bg-white/[0.03] p-4 text-left text-white transition hover:bg-white/[0.05]"
                    >
                      <div className="rounded-2xl border border-white/10 bg-white/5 p-2 text-slate-200">
                        <FiActivity className="size-4" />
                      </div>
                      <div>
                        <div className="font-medium">Technical Details</div>
                        <div className="mt-1 text-sm text-slate-400">Wallpaper id, entrypoint, and monitor metadata.</div>
                      </div>
                    </button>
                    {selectedAsset?.compatibility.warnings.length ? (
                      <div className="rounded-[22px] border border-amber-300/20 bg-amber-300/10 p-4 text-sm text-amber-100">
                        <div className="mb-2 font-medium">Compatibility Notes</div>
                        <div className="space-y-2">
                          {selectedAsset.compatibility.warnings.slice(0, 3).map((warning) => (
                            <div key={warning}>{warning}</div>
                          ))}
                        </div>
                      </div>
                    ) : null}
                    {selectedAsset?.kind === 'scene' && selectedAsset.source_kind === 'native' ? (
                      <button
                        type="button"
                        disabled={busy === `load-scene:${selectedAsset.id}` || snapshot?.source !== 'tauri'}
                        onClick={() => {
                          void openComposerForExistingScene(selectedAsset)
                        }}
                        className="inline-flex w-full items-center justify-center gap-2 rounded-2xl border border-white/10 bg-white/[0.04] px-4 py-3 text-sm font-medium text-white transition hover:bg-white/[0.08] disabled:cursor-not-allowed disabled:opacity-60"
                      >
                        <FiSliders className={busy === `load-scene:${selectedAsset.id}` ? 'size-4 animate-spin' : 'size-4'} />
                        {busy === `load-scene:${selectedAsset.id}` ? 'Opening scene...' : 'Edit scene'}
                      </button>
                    ) : null}
                    {workshopEnabled && selectedAsset?.source_kind === 'wallpaper_engine_import' && selectedAsset.import_metadata?.source_path ? (
                      <div className="grid gap-3 sm:grid-cols-2">
                        <button
                          type="button"
                          disabled={busy === `reimport:${selectedAsset.id}` || snapshot?.source !== 'tauri'}
                          onClick={() => {
                            void handleReimportAsset(selectedAsset.id)
                          }}
                          className="inline-flex w-full items-center justify-center gap-2 rounded-2xl border border-white/10 bg-white/[0.04] px-4 py-3 text-sm font-medium text-white transition hover:bg-white/[0.08] disabled:cursor-not-allowed disabled:opacity-60"
                        >
                          <FiRefreshCw className={busy === `reimport:${selectedAsset.id}` ? 'size-4 animate-spin' : 'size-4'} />
                          {busy === `reimport:${selectedAsset.id}` ? 'Reimporting...' : 'Reimport from source'}
                        </button>
                        <button
                          type="button"
                          disabled={busy === `remove:${selectedAsset.id}` || snapshot?.source !== 'tauri'}
                          onClick={() => {
                            void handleRemoveAsset(selectedAsset.id)
                          }}
                          className="inline-flex w-full items-center justify-center gap-2 rounded-2xl border border-rose-300/20 bg-rose-400/10 px-4 py-3 text-sm font-medium text-rose-100 transition hover:bg-rose-400/20 disabled:cursor-not-allowed disabled:opacity-60"
                        >
                          <FiX className="size-4" />
                          {busy === `remove:${selectedAsset.id}` ? 'Removing...' : 'Remove import'}
                        </button>
                      </div>
                    ) : null}
                    </div>
                  </div>
                </div>
            </div>
          </section>
        </div>
      </motion.div>

      {assetContextMenu && contextMenuAsset ? (
        <div
          className="fixed z-[60] min-w-[220px] overflow-hidden border border-white/10 bg-[var(--panel-bg)] shadow-[0_12px_28px_rgba(15,23,42,0.24)]"
          onClick={(event) => event.stopPropagation()}
          onPointerDown={(event) => event.stopPropagation()}
          onContextMenu={(event) => event.preventDefault()}
          style={{
            left: Math.min(assetContextMenu.x, window.innerWidth - 236),
            top: Math.min(assetContextMenu.y, window.innerHeight - 220),
          }}
        >
          <div className="border-b border-white/10 px-3 py-2">
            <div className="truncate text-sm font-medium text-white">{contextMenuAsset.name}</div>
            <div className="mt-1 truncate text-xs text-slate-400">{contextMenuAsset.id}</div>
          </div>
          <div className="p-1.5">
            {selectedMonitor ? (
              <button
                className="flex w-full items-center justify-between px-3 py-2 text-left text-sm text-slate-200 transition hover:bg-white/[0.06]"
                onClick={() => {
                  setAssetContextMenu(null)
                  void handleAssignWallpaper(selectedMonitor.id, contextMenuAsset.id)
                }}
                type="button"
              >
                <span>Apply to {selectedMonitor.output_name}</span>
                <FiDisc className="size-4" />
              </button>
            ) : null}
            {contextMenuAsset.kind === 'image' ? (
              <button
                className="flex w-full items-center justify-between px-3 py-2 text-left text-sm text-slate-200 transition hover:bg-white/[0.06]"
                onClick={() => {
                  setAssetContextMenu(null)
                  openComposer(contextMenuAsset)
                }}
                type="button"
              >
                <span>Create Scene</span>
                <FiImage className="size-4" />
              </button>
            ) : null}
            {contextMenuAsset.kind === 'scene' && contextMenuAsset.source_kind === 'native' ? (
              <button
                className="flex w-full items-center justify-between px-3 py-2 text-left text-sm text-slate-200 transition hover:bg-white/[0.06]"
                onClick={() => {
                  setAssetContextMenu(null)
                  void openComposerForExistingScene(contextMenuAsset)
                }}
                type="button"
              >
                <span>Edit Scene</span>
                <FiSliders className="size-4" />
              </button>
            ) : null}
            {isUserManagedAsset(contextMenuAsset) ? (
              <button
                className="flex w-full items-center justify-between px-3 py-2 text-left text-sm text-rose-100 transition hover:bg-rose-400/10"
                onClick={() => {
                  setAssetContextMenu(null)
                  void handleRemoveAsset(contextMenuAsset.id)
                }}
                type="button"
              >
                <span>{contextMenuAsset.source_kind === 'wallpaper_engine_import' ? 'Remove import' : 'Delete wallpaper'}</span>
                <FiTrash2 className="size-4" />
              </button>
            ) : null}
          </div>
        </div>
      ) : null}

      {monitorPickerOpen ? (
        <div className="absolute inset-0 z-50 flex items-center justify-center bg-black/55 p-6 backdrop-blur-sm">
          <div className="glass-panel w-full max-w-[560px] overflow-hidden">
            <div className="flex items-center justify-between border-b border-white/10 px-5 py-4">
              <div>
                <div className="section-title">Monitors</div>
                <div className="mt-1 text-sm text-slate-300">Choose which output receives assignments.</div>
              </div>
              <button
                className="icon-button"
                onClick={() => setMonitorPickerOpen(false)}
                type="button"
              >
                <FiX className="size-4" />
              </button>
            </div>
            <div className="space-y-3 px-5 py-5">
              {monitors.map((monitor) => {
                const active = selectedMonitor?.id === monitor.id
                const assignment = assignments.find(
                  ({ monitor_id }) =>
                    monitor_id === monitor.id || monitor_id === monitor.output_name,
                )
                return (
                  <button
                    key={monitor.id}
                    type="button"
                    onClick={() => {
                      setSelectedMonitorId(monitor.id)
                      setMonitorPickerOpen(false)
                    }}
                    className={[
                      'w-full border px-4 py-3 text-left transition',
                      active
                        ? 'border-cyan-300/40 bg-cyan-300/10 shadow-[0_0_0_1px_rgba(110,242,200,0.12)]'
                        : 'border-white/10 bg-white/[0.035] hover:bg-white/[0.06]',
                    ].join(' ')}
                  >
                    <div className="flex items-start justify-between gap-3">
                      <div>
                        <div className="text-base font-semibold text-white">{monitor.output_name}</div>
                        <div className="mt-1 text-xs uppercase tracking-[0.18em] text-slate-400">
                          {monitor.width}x{monitor.height}
                        </div>
                      </div>
                      <span className={monitor.focused ? 'control-chip border-emerald-300/30 text-emerald-200' : 'control-chip'}>
                        {monitor.focused ? 'Focused' : 'Idle'}
                      </span>
                    </div>
                    <div className="mt-4 text-sm text-slate-300">
                      {assignment ? assignment.wallpaper.name : 'No wallpaper assigned'}
                    </div>
                  </button>
                )
              })}
            </div>
          </div>
        </div>
      ) : null}

      {settingsOpen ? (
        <div className="absolute inset-0 z-50 flex items-center justify-center bg-black/55 p-6 backdrop-blur-sm">
          <div className="glass-panel w-full max-w-[560px] overflow-hidden">
            <div className="flex items-center justify-between border-b border-white/10 px-5 py-4">
              <div>
                <div className="section-title">Settings</div>
                <div className="mt-1 text-sm text-slate-300">Runtime policy and daemon status.</div>
              </div>
              <button
                className="icon-button"
                onClick={() => setSettingsOpen(false)}
                type="button"
              >
                <FiX className="size-4" />
              </button>
            </div>
            <div className="space-y-4 px-5 py-5">
              {snapshot ? (
                <form
                  className="space-y-4"
                  onSubmit={(event) => {
                    event.preventDefault()
                    const formData = new FormData(event.currentTarget)
                    void handlePausePolicyChange({
                      pause_on_fullscreen: formData.get('pause_on_fullscreen') === 'on',
                      pause_on_battery: formData.get('pause_on_battery') === 'on',
                      fps_limit: Number(formData.get('fps_limit') ?? snapshot.state.pause.fps_limit),
                    })
                  }}
                >
                  <ThemePreferenceControl
                    activeTheme={activeTheme}
                    preference={themePreference}
                    onChange={setThemePreference}
                  />
                  <PolicyToggle
                    defaultChecked={snapshot.state.pause.pause_on_fullscreen}
                    label="Pause on fullscreen"
                    name="pause_on_fullscreen"
                  />
                  <PolicyToggle
                    defaultChecked={snapshot.state.pause.pause_on_battery}
                    label="Pause on battery"
                    name="pause_on_battery"
                  />
                  <label className="block">
                    <span className="mb-2 block text-xs uppercase tracking-[0.18em] text-slate-400">FPS limit</span>
                    <input
                      className="field-shell w-full"
                      defaultValue={snapshot.state.pause.fps_limit}
                      min={1}
                      max={240}
                      name="fps_limit"
                      type="number"
                    />
                  </label>
                  <button
                    disabled={busy === 'pause' || snapshot.source !== 'tauri'}
                    type="submit"
                    className="primary-button w-full disabled:cursor-not-allowed disabled:opacity-60"
                  >
                    <FiActivity className="size-4" />
                    {busy === 'pause' ? 'Saving policy...' : 'Save policy'}
                  </button>
                </form>
              ) : null}

              {(uiError || snapshot?.error) ? (
                <div className="tone-danger p-4 text-sm">
                  <div className="mb-2 flex items-center gap-2 font-medium">
                    <FiAlertCircle className="size-4" />
                    Runtime message
                  </div>
                  <div>{uiError ?? snapshot?.error}</div>
                </div>
              ) : null}

              <div className="surface-panel p-4">
                <div className="mb-3 flex items-center justify-between">
                  <p className="section-title">Runtime Dependencies</p>
                  <span className={`control-chip ${snapshot?.state.runtime_dependencies.video.available ? 'border-emerald-300/30 text-emerald-200' : 'border-amber-300/30 text-amber-200'}`}>
                    {snapshot?.state.runtime_dependencies.video.available ? 'video deps found' : 'preview fallback'}
                  </span>
                </div>
                <div className="surface-muted p-3">
                  <div className="text-sm font-medium text-white">Video runtime</div>
                  <div className="mt-1 text-xs uppercase tracking-[0.16em] text-slate-500">
                    {snapshot?.state.runtime_dependencies.video.mode ?? 'unknown'}
                  </div>
                  <div className="mt-2 text-sm leading-6 text-slate-300">
                    {snapshot?.state.runtime_dependencies.video.detail
                      ?? 'No dependency details reported.'}
                  </div>
                </div>
              </div>

              <div className="surface-panel p-4">
                <div className="mb-3 flex items-center justify-between">
                  <p className="section-title">Recent Runtime</p>
                  {unresolvedAssignments.length > 0 ? (
                    <span className="control-chip border-amber-300/30 text-amber-200">
                      unresolved {unresolvedAssignments.length}
                    </span>
                  ) : null}
                </div>
                <div className="space-y-3">
                  {recentEvents.length === 0 ? (
                    <EmptyState text="Pause, resume, assignment, and restart events will appear here." />
                  ) : (
                    [...recentEvents].reverse().slice(0, 6).map((event, index) => (
                      <motion.div
                        key={`${event.runtime_key}:${index}`}
                        initial={{ opacity: 0, y: 8 }}
                        animate={{ opacity: 1, y: 0 }}
                        className="surface-muted p-3"
                      >
                        <div className="text-sm font-medium text-white">{event.message}</div>
                        <div className="mt-1 text-xs text-slate-400">{event.runtime_key}</div>
                      </motion.div>
                    ))
                  )}
                </div>
              </div>
            </div>
          </div>
        </div>
      ) : null}

      {advancedOpen ? (
        <div className="absolute inset-0 z-50 flex items-center justify-center bg-black/55 p-6 backdrop-blur-sm">
          <div className="glass-panel w-full max-w-[560px] overflow-hidden">
            <div className="flex items-center justify-between border-b border-white/10 px-5 py-4">
              <div>
                <div className="section-title">Technical Details</div>
                <div className="mt-1 text-sm text-slate-300">Low-level wallpaper and monitor metadata.</div>
              </div>
              <button
                className="icon-button"
                onClick={() => setAdvancedOpen(false)}
                type="button"
              >
                <FiX className="size-4" />
              </button>
            </div>
            <div className="space-y-3 px-5 py-5">
              {[
                `wallpaper id: ${selectedAsset?.id ?? 'n/a'}`,
                `entrypoint: ${selectedAsset?.entrypoint ?? 'n/a'}`,
                `source: ${selectedAsset?.source_kind ?? 'n/a'}`,
                selectedAsset?.kind === 'image' && selectedAsset.image_fit
                  ? `default fit: ${selectedAsset.image_fit}`
                  : selectedAsset?.kind === 'shader'
                    ? `mode: ${selectedAsset.animated ? 'animated shader' : 'static shader'}`
                    : 'mode: n/a',
                selectedMonitor ? `monitor id: ${selectedMonitor.id}` : 'monitor id: n/a',
                workshopEnabled && selectedAsset?.import_metadata?.source_path ? `import path: ${selectedAsset.import_metadata.source_path}` : null,
                workshopEnabled && selectedAsset?.import_metadata?.manifest_path ? `manifest: ${selectedAsset.import_metadata.manifest_path}` : null,
                workshopEnabled && selectedAsset?.import_metadata?.workshop_id ? `workshop id: ${selectedAsset.import_metadata.workshop_id}` : null,
                workshopEnabled && selectedAsset?.import_metadata?.original_type ? `original type: ${selectedAsset.import_metadata.original_type}` : null,
              ].map((line) => (
                line ? (
                <div key={line} className="surface-muted break-words px-4 py-3 text-sm text-slate-300">
                  {line}
                </div>
                ) : null
              ))}
            </div>
          </div>
        </div>
      ) : null}

      <input
        ref={createImageFileInputRef}
        accept="image/png,image/jpeg,image/webp,image/x-portable-pixmap"
        className="hidden"
        onChange={(event) => {
          void handleCreateNativeAssetFilePicked(event, 'image')
        }}
        type="file"
      />
      <input
        ref={createVideoFileInputRef}
        accept="video/mp4,video/webm,video/quicktime,.mkv"
        className="hidden"
        onChange={(event) => {
          void handleCreateNativeAssetFilePicked(event, 'video')
        }}
        type="file"
      />
      <input
        ref={createShaderFileInputRef}
        accept=".wgsl,text/plain"
        className="hidden"
        onChange={(event) => {
          void handleCreateNativeAssetFilePicked(event, 'shader')
        }}
        type="file"
      />

      {createPickerOpen ? (
        <div className="absolute inset-0 z-50 flex items-center justify-center bg-black/55 p-6 backdrop-blur-sm">
          <div className="glass-panel w-full max-w-[860px] overflow-hidden">
            <div className="flex items-center justify-between border-b border-white/10 px-5 py-4">
              <div>
                <div className="section-title">Create Wallpaper</div>
                <div className="mt-1 text-sm text-slate-300">Choose a native wallpaper type to add to the library.</div>
              </div>
              <button
                className="icon-button"
                onClick={() => setCreatePickerOpen(false)}
                type="button"
              >
                <FiX className="size-4" />
              </button>
            </div>
            <div className="border-b border-white/10 px-5 py-4">
              <label className="block">
                <span className="mb-2 block text-xs uppercase tracking-[0.18em] text-slate-400">Wallpaper name</span>
                <input
                  className="field-shell w-full"
                  onChange={(event) => setCreateName(event.currentTarget.value)}
                  placeholder="Optional. Defaults to the file name."
                  type="text"
                  value={createName}
                />
              </label>
            </div>
            <div className="grid gap-4 p-5 md:grid-cols-2">
              {[
                {
                  kind: 'image' as const,
                  icon: <FiImage className="size-5" />,
                  title: 'Still Image',
                  detail: 'Import a local image as a managed native wallpaper asset.',
                  action: () => createImageFileInputRef.current?.click(),
                },
                {
                  kind: 'scene' as const,
                  icon: <FiActivity className="size-5" />,
                  title: 'Scene',
                  detail: 'Open the Scene Composer to build a native real-time scene wallpaper.',
                  action: () => {
                    setCreatePickerOpen(false)
                    openComposer(undefined, createName.trim() || undefined)
                  },
                },
                {
                  kind: 'shader' as const,
                  icon: <FiZap className="size-5" />,
                  title: 'Shader',
                  detail: 'Import a local WGSL shader as a managed native shader wallpaper asset.',
                  action: () => createShaderFileInputRef.current?.click(),
                },
                {
                  kind: 'video' as const,
                  icon: <FiFilm className="size-5" />,
                  title: 'Video',
                  detail: 'Import a local video file as a managed native video wallpaper asset.',
                  action: () => createVideoFileInputRef.current?.click(),
                },
              ].map((option) => (
                <button
                  key={option.kind}
                  className="surface-panel flex min-h-[152px] flex-col items-start gap-4 p-5 text-left transition hover:border-white/20 hover:bg-white/[0.05]"
                  disabled={Boolean(busy)}
                  onClick={option.action}
                  type="button"
                >
                  <div className="flex size-11 items-center justify-center border border-white/10 bg-white/[0.04] text-slate-100">
                    {option.icon}
                  </div>
                  <div>
                    <div className="text-base font-semibold text-white">{option.title}</div>
                    <div className="mt-2 text-sm leading-6 text-slate-400">{option.detail}</div>
                  </div>
                </button>
              ))}
            </div>
          </div>
        </div>
      ) : null}

      {workshopEnabled && importOpen ? (
        <div className="absolute inset-0 z-50 flex items-center justify-center bg-black/55 p-6 backdrop-blur-sm">
          <div className="glass-panel w-full max-w-[640px] overflow-hidden">
            <div className="flex items-center justify-between border-b border-white/10 px-5 py-4">
              <div>
                <div className="section-title">Import Workshop Item</div>
                <div className="mt-1 text-sm text-slate-300">Import a local Wallpaper Engine workshop item or workshop directory.</div>
              </div>
              <button
                className="icon-button"
                onClick={() => setImportOpen(false)}
                type="button"
              >
                <FiX className="size-4" />
              </button>
            </div>
            <div className="space-y-4 px-5 py-5">
              {suggestedWorkshopPaths.length > 0 ? (
                <div className="surface-panel p-4">
                  <div className="mb-3 text-xs uppercase tracking-[0.18em] text-slate-400">Detected workshop roots</div>
                  <div className="flex flex-col gap-2">
                    {suggestedWorkshopPaths.map((path) => (
                      <button
                        key={path}
                        type="button"
                        onClick={() => setImportPath(path)}
                        className="secondary-button justify-start px-3 py-2 text-left text-sm font-normal text-slate-200"
                      >
                        {path}
                      </button>
                    ))}
                  </div>
                </div>
              ) : null}
              <label className="block">
                <span className="mb-2 block text-xs uppercase tracking-[0.18em] text-slate-400">Local path</span>
                <input
                  className="field-shell w-full"
                  onChange={(event) => setImportPath(event.currentTarget.value)}
                  placeholder="/home/chris/.local/share/Steam/steamapps/workshop/content/431960/1234567890"
                  type="text"
                  value={importPath}
                />
              </label>
              <div className="surface-panel p-4 text-sm leading-6 text-slate-300">
                Backlayer currently imports local Wallpaper Engine items into its own managed storage and classifies them as video, scene, or web wallpapers.
              </div>
              {uiError ? (
                <div className="rounded-2xl border border-rose-300/20 bg-rose-400/10 p-4 text-sm text-rose-100">
                  <div className="mb-2 flex items-center gap-2 font-medium">
                    <FiAlertCircle className="size-4" />
                    Import error
                  </div>
                  <div>{uiError}</div>
                </div>
              ) : null}
              {importResult.length > 0 ? (
                <div className="tone-info p-4 text-sm">
                  <div className="mb-2 flex items-center gap-2 font-medium">
                    <FiDownload className="size-4" />
                    Import complete
                  </div>
                  <div>
                    Imported {importResult.length} item{importResult.length === 1 ? '' : 's'}:
                  </div>
                  <div className="mt-2 flex flex-wrap gap-2">
                    {importResult.slice(0, 6).map((asset) => (
                      <button
                        key={asset.id}
                        type="button"
                        onClick={() => setSelectedAssetId(asset.id)}
                        className="control-chip border-emerald-300/30 text-emerald-100"
                      >
                        {asset.name}
                      </button>
                    ))}
                  </div>
                </div>
              ) : null}
              <button
                disabled={busy === 'import' || snapshot?.source !== 'tauri'}
                onClick={() => {
                  void handleImportWorkshopPath()
                }}
                type="button"
                className="primary-button w-full disabled:cursor-not-allowed disabled:opacity-60"
              >
                <FiDownload className="size-4" />
                {busy === 'import' ? 'Importing...' : 'Import workshop path'}
              </button>
              {importResult.length > 0 ? (
                <button
                  type="button"
                  onClick={() => {
                    setImportOpen(false)
                    setImportPath('')
                    setImportResult([])
                    setUiError(null)
                  }}
                  className="secondary-button w-full"
                >
                  Done
                </button>
              ) : null}
            </div>
          </div>
        </div>
      ) : null}

      {composerOpen ? (
        <div className="absolute inset-0 z-50 bg-black/70 backdrop-blur-sm">
          <input
            ref={composerFileInputRef}
            accept="image/png,image/jpeg,image/webp,image/x-portable-pixmap"
            className="hidden"
            onChange={(event) => {
              void handleComposerFilePicked(event)
            }}
            type="file"
          />
          <input
            ref={composerSpriteFileInputRef}
            accept="image/png,image/jpeg,image/webp,image/x-portable-pixmap"
            className="hidden"
            onChange={(event) => {
              void handleComposerSpriteFilePicked(event)
            }}
            type="file"
          />
          <div className="glass-panel grid h-full w-full min-h-0 grid-rows-[auto_minmax(0,1fr)] rounded-none border-0">
            <div className="flex items-center justify-between gap-4 border-b border-white/10 px-5 py-3">
              <div className="min-w-0">
                <div className="section-title">Scene Composer</div>
                <div className="mt-1 flex flex-wrap items-center gap-2 text-sm text-slate-300">
                  <span>{composerEditingAssetId ? 'Editing native scene asset.' : 'Create a native Backlayer scene from an image source.'}</span>
                  {composerSourceName ? <span className="control-chip">{composerSourceName}</span> : null}
                </div>
              </div>
              <div className="flex min-w-0 flex-1 items-center justify-end gap-3">
                <button
                  className="secondary-button shrink-0"
                  onClick={() => setComposerLeftTab('add')}
                  type="button"
                >
                  <FiSliders className="size-4" />
                  Add
                </button>
                <label className="min-w-[240px] max-w-[360px] flex-1">
                  <span className="sr-only">Scene name</span>
                  <input
                    className="field-shell w-full"
                    onChange={(event) => setComposerName(event.currentTarget.value)}
                    placeholder="Scene name"
                    type="text"
                    value={composerName}
                  />
                </label>
                <button
                  className="primary-button shrink-0 disabled:cursor-not-allowed disabled:opacity-60"
                  disabled={composerLoadingScene || busy === 'create-scene' || (!composerBaseAsset && !composerUploadImage)}
                  onClick={() => {
                    void handleCreateScene()
                  }}
                  type="button"
                >
                  <FiDisc className="size-4" />
                  {busy === 'create-scene' ? 'Creating…' : 'Save Scene'}
                </button>
                <button className="icon-button shrink-0" onClick={closeComposer} type="button">
                  <FiX className="size-4" />
                </button>
              </div>
            </div>

            <div className="grid min-h-0 gap-4 p-4 xl:grid-cols-[300px_minmax(0,1fr)_340px]">
              <div className="surface-panel flex min-h-0 flex-col overflow-hidden">
                <div className="flex shrink-0 items-center gap-2 border-b border-white/10 px-3 py-3">
                  {([
                    ['layers', 'Layers'],
                    ['assets', 'Assets'],
                    ['add', 'Add'],
                  ] as const).map(([tab, label]) => (
                    <button
                      key={tab}
                      className={[
                        'secondary-button min-w-0 flex-1 justify-center px-3 py-2',
                        composerLeftTab === tab ? 'border-cyan-300/40 text-cyan-100' : '',
                      ].join(' ')}
                      onClick={() => setComposerLeftTab(tab)}
                      type="button"
                    >
                      {label}
                    </button>
                  ))}
                </div>

                <div className="min-h-0 flex-1 overflow-y-auto p-4">
                  {composerLeftTab === 'layers' ? (
                    <div className="space-y-3">
                      <div className="flex items-center justify-between">
                        <div>
                          <div className="section-title">Layers</div>
                          <div className="mt-1 text-sm text-slate-400">Select, reorder, and remove scene nodes.</div>
                        </div>
                        <span className="control-chip">{composerNodes.length} nodes</span>
                      </div>
                      <div className="grid gap-2">
                        {composerNodes.map((node, index) => (
                          <button
                            key={node.id}
                            className={[
                              'surface-muted flex w-full items-center justify-between px-3 py-3 text-left',
                              selectedComposerNode?.id === node.id ? 'border border-cyan-300/40 shadow-[inset_0_0_0_1px_rgba(125,211,252,0.1)]' : 'border border-transparent',
                            ].join(' ')}
                            onClick={() => setSelectedComposerNodeId(node.id)}
                            type="button"
                          >
                            <div className="min-w-0">
                              <div className="truncate text-sm font-medium text-slate-100">{node.name}</div>
                              <div className="mt-1 text-[11px] uppercase tracking-[0.16em] text-slate-400">{node.kind}</div>
                            </div>
                            <div className="ml-3 flex shrink-0 items-center gap-1">
                              <button
                                className="icon-button"
                                disabled={index === 0}
                                onClick={(event) => {
                                  event.stopPropagation()
                                  setComposerNodes((items) => moveComposerNode(items, index, index - 1))
                                }}
                                type="button"
                              >
                                ↑
                              </button>
                              <button
                                className="icon-button"
                                disabled={index === composerNodes.length - 1}
                                onClick={(event) => {
                                  event.stopPropagation()
                                  setComposerNodes((items) => moveComposerNode(items, index, index + 1))
                                }}
                                type="button"
                              >
                                ↓
                              </button>
                              {node.kind !== 'sprite' || node.id !== 'sprite-base' ? (
                                <button
                                  className="icon-button"
                                  onClick={(event) => {
                                    event.stopPropagation()
                                    setComposerNodes((items) => items.filter((item) => item.id !== node.id))
                                  }}
                                  type="button"
                                >
                                  <FiTrash2 className="size-4" />
                                </button>
                              ) : null}
                            </div>
                          </button>
                        ))}
                      </div>
                    </div>
                  ) : null}

                  {composerLeftTab === 'assets' ? (
                    <div className="space-y-4">
                      <div>
                        <div className="section-title">Assets</div>
                        <div className="mt-1 text-sm text-slate-400">Manage the base image and any imported overlay images.</div>
                      </div>
                      <div className="surface-muted p-4 text-sm leading-6 text-slate-300">
                        <div className="text-[11px] uppercase tracking-[0.16em] text-slate-500">Current source</div>
                        <div className="mt-2 font-medium text-slate-100">{composerSourceName ?? 'No image selected yet'}</div>
                        <div className="mt-1 text-slate-400">
                          {composerUploadImage
                            ? `Local file: ${composerUploadImage.filename}`
                            : composerBaseAsset
                              ? `Library asset: ${composerBaseAsset.id}`
                              : 'Pick a file or use the currently selected image wallpaper.'}
                        </div>
                      </div>
                      <div className="grid gap-3">
                        <button className="secondary-button w-full justify-center" onClick={handleUseSelectedImage} type="button">
                          <FiImage className="size-4" />
                          Use selected wallpaper
                        </button>
                        <button
                          className="secondary-button w-full justify-center"
                          onClick={() => composerFileInputRef.current?.click()}
                          type="button"
                        >
                          <FiDownload className="size-4" />
                          Choose image file
                        </button>
                        <button
                          className="secondary-button w-full justify-center"
                          onClick={() => composerSpriteFileInputRef.current?.click()}
                          type="button"
                        >
                          <FiImage className="size-4" />
                          Add overlay image
                        </button>
                      </div>
                      <div className="space-y-2">
                        <div className="section-title">Overlay images</div>
                        {composerExtraImages.length > 0 ? (
                          <div className="grid gap-2">
                            {composerExtraImages.map((image) => (
                              <div key={image.key} className="surface-muted flex items-center justify-between px-3 py-3 text-sm">
                                <div className="min-w-0">
                                  <div className="truncate font-medium text-slate-100">{image.name}</div>
                                  <div className="mt-1 text-[11px] uppercase tracking-[0.16em] text-slate-400">{image.key}</div>
                                </div>
                                <button
                                  className="icon-button"
                                  onClick={() => {
                                    setComposerExtraImages((items) => items.filter((item) => item.key !== image.key))
                                    setComposerNodes((items) => items.filter((item) => !(item.kind === 'sprite' && item.image_key === image.key)))
                                  }}
                                  type="button"
                                >
                                  <FiTrash2 className="size-4" />
                                </button>
                              </div>
                            ))}
                          </div>
                        ) : (
                          <div className="surface-muted p-4 text-sm text-slate-400">
                            No overlay images yet. Add one to build layered scenes.
                          </div>
                        )}
                      </div>
                    </div>
                  ) : null}

                  {composerLeftTab === 'add' ? (
                    <div className="space-y-4">
                      <div>
                        <div className="section-title">Add Nodes</div>
                        <div className="mt-1 text-sm text-slate-400">Insert new scene content, effects, and emitters.</div>
                      </div>
                      <div className="grid gap-3">
                        <button
                          className="secondary-button w-full justify-center"
                          onClick={() => composerSpriteFileInputRef.current?.click()}
                          type="button"
                        >
                          <FiImage className="size-4" />
                          Overlay Image
                        </button>
                        {SCENE_EFFECT_OPTIONS.map((option) => (
                          <button
                            key={option.effect}
                            className="secondary-button w-full justify-between"
                            onClick={() => {
                              const node = createComposerEffectNode(option.effect)
                              setComposerNodes((items) => [...items, node])
                              setSelectedComposerNodeId(node.id)
                              setComposerLeftTab('layers')
                            }}
                            type="button"
                          >
                            <span className="inline-flex items-center gap-2">
                              <FiZap className="size-4" />
                              {option.label}
                            </span>
                            <span className="text-[11px] uppercase tracking-[0.16em] text-slate-400">Effect</span>
                          </button>
                        ))}
                        {SCENE_EMITTER_OPTIONS.map((option) => (
                          <button
                            key={option.preset}
                            className="secondary-button w-full justify-between"
                            onClick={() => {
                              const node = createComposerEmitterNode(option.preset)
                              setComposerNodes((items) => [...items, node])
                              setSelectedComposerNodeId(node.id)
                              setComposerLeftTab('layers')
                            }}
                            type="button"
                          >
                            <span className="inline-flex items-center gap-2">
                              <FiActivity className="size-4" />
                              {option.label}
                            </span>
                            <span className="text-[11px] uppercase tracking-[0.16em] text-slate-400">Emitter</span>
                          </button>
                        ))}
                        <button
                          className="secondary-button w-full justify-between"
                          onClick={() => {
                            const node = createComposerParticleAreaNode()
                            setComposerNodes((items) => [...items, node])
                            setSelectedComposerNodeId(node.id)
                            setComposerViewportTool('region')
                            setComposerLeftTab('layers')
                          }}
                          type="button"
                        >
                          <span className="inline-flex items-center gap-2">
                            <FiFilter className="size-4" />
                            Particle Area
                          </span>
                          <span className="text-[11px] uppercase tracking-[0.16em] text-slate-400">Area</span>
                        </button>
                      </div>
                    </div>
                  ) : null}
                </div>
              </div>

              <div className="surface-panel min-h-0 overflow-hidden">
                <div className="flex items-center justify-between border-b border-white/10 px-4 py-3">
                  <div>
                    <div className="text-sm font-medium text-white">Viewport</div>
                    <div className="mt-1 text-xs text-slate-400">
                      Edit in the viewport first. The selected node stays available in Properties.
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    {selectedComposerNode ? (
                      <div className="flex items-center gap-1">
                        {resolveViewportToolsForNode(selectedComposerNode).map((tool) => (
                          <button
                            key={tool}
                            className={[
                              'secondary-button px-2 py-1.5 text-xs uppercase tracking-[0.14em]',
                              composerViewportTool === tool ? 'border-cyan-300/40 text-cyan-100' : '',
                            ].join(' ')}
                            onClick={() => setComposerViewportTool(tool)}
                            type="button"
                          >
                            {viewportToolLabel(tool)}
                          </button>
                        ))}
                      </div>
                    ) : null}
                    {selectedComposerNode ? <span className="control-chip">{selectedComposerNode.kind}</span> : null}
                    {composerSourceName ? <span className="control-chip">{composerSourceName}</span> : null}
                  </div>
                </div>
                <div className="h-full min-h-0 p-4">
                  {composerLoadingScene ? (
                    <div className="flex h-full items-center justify-center">
                      <div className="surface-muted max-w-md p-6 text-center">
                        <div className="section-title">Loading scene</div>
                        <div className="mt-2 text-sm text-slate-400">
                          Opening the editor shell first, then hydrating the scene images and nodes.
                        </div>
                      </div>
                    </div>
                  ) : composerBaseAsset || composerUploadImage ? (
                    <ComposerEnginePreview
                      asset={composerBaseAsset}
                      documentVisible={documentVisible}
                      paused={particleEditorNode !== null}
                      nodes={composerNodes}
                      extraImages={composerExtraImages}
                      activeTool={composerViewportTool}
                      previewTargetSize={selectedMonitor ? {
                        width: selectedMonitor.width,
                        height: selectedMonitor.height,
                        label: selectedMonitor.output_name,
                      } : null}
                      uploadedImagePath={composerUploadImage?.path ?? null}
                      uploadedImageUrl={composerUploadImage?.sourceUrl ?? null}
                      selectedNode={selectedComposerNode}
                      onSelectNode={(nodeId) => setSelectedComposerNodeId(nodeId)}
                      onChangeNode={(nextNode) => {
                        setComposerNodes((items) => items.map((item) => (item.id === nextNode.id ? nextNode : item)))
                      }}
                      saving={busy === 'create-scene'}
                    />
                  ) : (
                    <EmptyState text="Choose an image source in Assets to start composing a native scene." />
                  )}
                </div>
              </div>

              <div className="surface-panel flex min-h-0 flex-col overflow-hidden">
                <div className="shrink-0 border-b border-white/10 px-5 py-4">
                  <div className="section-title">Properties</div>
                  {selectedComposerNode ? (
                    <div className="mt-2">
                      <div className="text-sm font-medium text-white">{selectedComposerNode.name}</div>
                      <div className="mt-1 flex items-center gap-2 text-xs uppercase tracking-[0.16em] text-slate-400">
                        <span>{selectedComposerNode.kind}</span>
                        <span className="control-chip">{selectedComposerNode.enabled ? 'Enabled' : 'Disabled'}</span>
                      </div>
                    </div>
                  ) : (
                    <div className="mt-2 text-sm text-slate-400">Select a node from Layers to edit its properties.</div>
                  )}
                </div>
                <div className="min-h-0 flex-1 overflow-y-auto p-4">
                  {selectedComposerNode ? (
                    <ComposerNodeInspector
                      availableImageKeys={[
                        { key: 'base', label: composerSourceName ?? 'Base image' },
                        ...composerExtraImages.map((image) => ({ key: image.key, label: image.name })),
                      ]}
                      viewportTool={composerViewportTool}
                      node={selectedComposerNode}
                      onOpenParticleEditor={(nodeId) => setParticleEditorNodeId(nodeId)}
                      onActivateViewportTool={(nodeId, tool) => {
                        setSelectedComposerNodeId(nodeId)
                        setComposerViewportTool(tool)
                      }}
                      onChange={(nextNode) => {
                        setComposerNodes((items) => items.map((item) => (item.id === nextNode.id ? nextNode : item)))
                      }}
                    />
                  ) : (
                    <div className="surface-muted p-4 text-sm text-slate-400">No node selected.</div>
                  )}

                  {(uiError && busy === 'create-scene') ? (
                    <div className="tone-danger mt-4 p-4 text-sm">{uiError}</div>
                  ) : null}
                </div>
              </div>
            </div>

            {busy === 'create-scene' ? (
              <div className="absolute inset-0 z-10 flex items-center justify-center bg-slate-950/68 backdrop-blur-sm">
                <div className="surface-panel w-full max-w-[360px] p-6 text-center">
                  <div className="mx-auto mb-4 flex size-12 items-center justify-center rounded-full border border-cyan-300/30 bg-cyan-300/10 text-cyan-200">
                    <FiRefreshCw className="size-5 animate-spin" />
                  </div>
                  <div className="text-base font-semibold text-white">Generating scene assets</div>
                  <div className="mt-2 text-sm leading-6 text-slate-300">
                    Backlayer is writing the scene document, packaging source images, and generating a library preview.
                  </div>
                </div>
              </div>
            ) : null}

            {particleEditorNode ? (
              <ParticleEditorModal
                availableImageKeys={[
                  { key: 'base', label: composerSourceName ?? 'Base image' },
                  ...composerExtraImages.map((image) => ({ key: image.key, label: image.name })),
                ]}
                documentVisible={documentVisible}
                node={particleEditorNode}
                onClose={() => setParticleEditorNodeId(null)}
                onSave={(nextNode) => {
                  setComposerNodes((items) => items.map((item) => (item.id === nextNode.id ? nextNode : item)))
                  setParticleEditorNodeId(null)
                }}
              />
            ) : null}
          </div>
        </div>
      ) : null}
    </main>
  )
}

export default App

function StartupScreen({ activeTheme }: { activeTheme: ActiveTheme }) {
  return (
    <main className="h-dvh overflow-hidden p-3" data-theme={activeTheme}>
      <div className="glass-panel flex h-full w-full items-center justify-center">
        <motion.div
          className="surface-panel w-full max-w-[420px] p-8 text-center"
          initial={{ opacity: 0, y: 14 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.35, ease: 'easeOut' }}
        >
          <div className="mx-auto mb-5 flex size-14 items-center justify-center rounded-full border border-cyan-300/25 bg-cyan-300/10 text-cyan-200">
            <FiRefreshCw className="size-5 animate-spin" />
          </div>
          <div className="section-title">Backlayer</div>
          <div className="mt-2 text-xl font-semibold tracking-[-0.03em] text-white">
            Loading runtime
          </div>
          <div className="mt-3 text-sm leading-6 text-slate-300">
            Connecting to the daemon, reading wallpapers, and preparing the manager UI.
          </div>
        </motion.div>
      </div>
    </main>
  )
}

function detectSystemTheme(): ActiveTheme {
  if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') {
    return 'light'
  }

  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

function readThemePreference(): ThemePreference {
  if (typeof window === 'undefined') {
    return 'system'
  }

  const stored = window.localStorage.getItem(THEME_PREFERENCE_KEY)
  if (stored === 'light' || stored === 'dark' || stored === 'system') {
    return stored
  }

  return 'system'
}

function buildDefaultComposerNodes(): SceneNode[] {
  return [
    {
      kind: 'sprite',
      id: 'sprite-base',
      name: 'Base image',
      enabled: true,
      image_key: 'base',
      fit: 'cover',
      blend: 'alpha',
      x: 0,
      y: 0,
      scale: 1,
      rotation_deg: 0,
      opacity: 1,
      particle_occluder: false,
      particle_surface: false,
      particle_region: null,
      behaviors: [],
    },
    {
      kind: 'effect',
      id: 'effect-glow-1',
      name: 'Glow',
      enabled: true,
      effect: 'glow',
      color_hex: defaultEffectColorHex('glow'),
      opacity: 0.48,
      intensity: 0.8,
      speed: 0.85,
    },
  ]
}

function createComposerEffectNode(effect: SceneEffectKind): SceneNode {
  return {
    kind: 'effect',
    id: `effect-${effect}-${Date.now().toString(36)}`,
    name: SCENE_EFFECT_OPTIONS.find((option) => option.effect === effect)?.label ?? 'Effect',
    enabled: true,
    effect,
    color_hex: defaultEffectColorHex(effect),
    opacity: effect === 'scanlines' ? 0.24 : 0.42,
    intensity: effect === 'glow' ? 0.9 : 0.75,
    speed: effect === 'scanlines' ? 1.25 : 0.8,
  }
}

function createComposerSpriteNode(imageKey: string, name: string): SceneNode {
  return {
    kind: 'sprite',
    id: `sprite-${imageKey}-${Date.now().toString(36)}`,
    name,
    enabled: true,
    image_key: imageKey,
    fit: 'contain',
    blend: 'screen',
    x: 0,
    y: 0,
    scale: 1,
    rotation_deg: 0,
    opacity: 0.92,
    particle_occluder: false,
    particle_surface: false,
    particle_region: null,
    behaviors: [{
      kind: 'drift',
      speed: 0.6,
      amount_x: 18,
      amount_y: 10,
      amount: 0,
      phase: 0,
    }],
  }
}

function createComposerEmitterNode(preset: SceneEmitterPreset): SceneNode {
  return {
    kind: 'emitter',
    id: `emitter-${preset}-${Date.now().toString(36)}`,
    name: SCENE_EMITTER_OPTIONS.find((option) => option.preset === preset)?.label ?? 'Emitter',
    enabled: true,
    preset,
    shape: defaultEmitterShape(preset),
    origin_x: defaultEmitterOriginX(preset),
    origin_y: defaultEmitterOriginY(preset),
    direction_deg: defaultEmitterDirectionDeg(preset),
    region_width: defaultEmitterRegionWidth(preset),
    region_height: defaultEmitterRegionHeight(preset),
    region_radius: defaultEmitterRegionRadius(preset),
    line_length: defaultEmitterLineLength(preset),
    line_angle_deg: defaultEmitterDirectionDeg(preset),
    emission_rate: preset === 'rain' ? 72 : preset === 'dust' ? 18 : preset === 'snow' ? 22 : 32,
    burst_count: preset === 'embers' ? 28 : preset === 'rain' ? 0 : preset === 'dust' ? 8 : 10,
    burst_on_start: preset !== 'rain',
    max_particles: preset === 'rain' ? 360 : preset === 'dust' ? 96 : preset === 'snow' ? 160 : 180,
    opacity: preset === 'dust' ? 0.35 : preset === 'snow' ? 0.5 : 0.6,
    size: preset === 'rain' ? 1.8 : preset === 'dust' ? 3.5 : preset === 'snow' ? 3.2 : 4.5,
    speed: preset === 'rain' ? 420 : preset === 'dust' ? 34 : preset === 'snow' ? 44 : 82,
    min_speed: defaultEmitterMinSpeed(preset),
    max_speed: defaultEmitterMaxSpeed(preset),
    min_life: defaultEmitterMinLife(preset),
    max_life: defaultEmitterMaxLife(preset),
    spread: preset === 'rain' ? 18 : preset === 'dust' ? 80 : preset === 'snow' ? 48 : 48,
    gravity_x: preset === 'rain' ? -48 : preset === 'snow' ? -14 : 0,
    gravity_y: preset === 'rain' ? 220 : preset === 'embers' ? -18 : preset === 'snow' ? 24 : -6,
    drag: preset === 'dust' ? 1.4 : preset === 'snow' ? 0.9 : 0.5,
    color_hex: defaultEmitterColorHex(preset),
    particle_image_key: null,
    particle_rotation_deg: 0,
    size_curve: defaultEmitterSizeCurve(preset),
    alpha_curve: defaultEmitterAlphaCurve(preset),
    color_curve: defaultEmitterColorCurve(preset),
  }
}

function createComposerParticleAreaNode(): SceneNode {
  return {
    kind: 'particle_area',
    id: `particle-area-${Date.now().toString(36)}`,
    name: 'Particle area',
    enabled: true,
    shape: 'rect',
    region: {
      x: 0.25,
      y: 0.25,
      width: 0.5,
      height: 0.2,
    },
    points: [],
    occluder: true,
    surface: false,
  }
}

function moveComposerNode(nodes: SceneNode[], from: number, to: number): SceneNode[] {
  if (from === to || from < 0 || to < 0 || from >= nodes.length || to >= nodes.length) {
    return nodes
  }
  const copy = [...nodes]
  const [item] = copy.splice(from, 1)
  copy.splice(to, 0, item)
  return copy
}

function slugifyComposerKey(value: string): string {
  const slug = value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
  return slug || 'sprite'
}

function uniqueComposerImageKey(baseKey: string, images: ComposerUploadImage[]): string {
  if (!images.some((image) => image.key === baseKey)) {
    return baseKey
  }
  for (let index = 2; index <= 999; index += 1) {
    const candidate = `${baseKey}-${index}`
    if (!images.some((image) => image.key === candidate)) {
      return candidate
    }
  }
  return `${baseKey}-${Date.now().toString(36)}`
}

function isUserManagedAsset(asset: AssetMetadata) {
  const assetPath = asset.asset_path ?? ''
  return (
    asset.source_kind === 'wallpaper_engine_import'
    || (asset.source_kind === 'native' && assetPath.includes('/.config/backlayer/assets/'))
  )
}

function assetSummary(asset: AssetMetadata) {
  if (asset.kind === 'shader') {
    return asset.animated
      ? 'Animated shader wallpaper running through the dedicated shader runner process.'
      : 'Static shader wallpaper for testing the real shader runtime without extra motion.'
  }

  if (asset.kind === 'image') {
    return `Image wallpaper using the ${asset.image_fit ?? 'cover'} fit mode.`
  }

  if (asset.kind === 'scene') {
    return asset.source_kind === 'wallpaper_engine_import'
      ? 'Imported Wallpaper Engine scene item. Classified and stored successfully, but scene playback is not implemented yet.'
      : 'Native real-time scene built from sprite, effect, and particle nodes in Backlayer Scene Composer.'
  }

  if (asset.kind === 'web') {
    return asset.source_kind === 'wallpaper_engine_import'
      ? 'Imported Wallpaper Engine web item. Classified and stored successfully, but web wallpaper playback is not implemented yet.'
      : 'Web wallpaper support is planned but not implemented.'
  }

  return asset.source_kind === 'wallpaper_engine_import'
    ? 'Imported Wallpaper Engine video item. Backlayer can now play video through the dedicated video runner, with libmpv and hardware decode still pending.'
    : 'Video wallpaper running through the dedicated FFmpeg-backed video runner.'
}

function importRuntimeNote(kind: AssetMetadata['kind']) {
  switch (kind) {
    case 'scene':
      return 'Imported Wallpaper Engine scene items are currently limited to static scene-image extraction or preview fallback. Full scene playback and animation are not implemented yet.'
    case 'web':
      return 'Imported Wallpaper Engine web items currently render a narrow static subset: a local HTML image, a parsed background color, or the preview fallback. Real browser-backed wallpaper playback is not implemented yet.'
    case 'video':
      return 'Imported Wallpaper Engine video items now use first-pass FFmpeg-backed playback in video-runner. libmpv integration and hardware decode are still not finished.'
    default:
      return 'Native Backlayer scene assets use a real-time scene graph with sprites, effects, and particle emitters.'
  }
}

function StatusPill({
  loading,
  source,
}: {
  loading: boolean
  source: RuntimeSnapshot['source'] | undefined
}) {
  if (loading) {
    return <span className="control-chip border-cyan-300/30 text-cyan-200">Loading runtime</span>
  }

  if (source === 'tauri') {
    return <span className="control-chip border-emerald-300/30 text-emerald-200">Connected to daemon</span>
  }

  if (source === 'tauri_disconnected') {
    return <span className="control-chip border-amber-300/30 text-amber-200">Daemon unavailable</span>
  }

  return <span className="control-chip border-slate-300/20 text-slate-300">Browser fallback</span>
}

function InspectorHero({
  asset,
  monitorName,
  runtimeLabel,
}: {
  asset: AssetMetadata | null
  monitorName: string
  runtimeLabel: string
}) {
  return (
    <div className="overflow-hidden rounded-[24px] border border-white/10 bg-white/[0.04]">
      <div className="h-28 border-b border-white/10 bg-slate-950/80">
        {asset ? <AssetPreview asset={asset} /> : null}
      </div>
      <div className="space-y-3 p-4">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="min-w-0">
            <div className="text-lg font-semibold tracking-[-0.03em] text-white">
              {asset?.name ?? 'No wallpaper selected'}
            </div>
            <div className="mt-1 text-sm leading-6 text-slate-300">
              {asset ? assetSummary(asset) : 'Select a wallpaper to see settings and assignment details.'}
            </div>
          </div>
          {asset ? (
            <span className="control-chip shrink-0">
              {asset.kind}
            </span>
          ) : null}
        </div>
        <div className="flex flex-wrap gap-2">
          <span className="control-chip">{monitorName}</span>
          <span className="control-chip">{runtimeLabel}</span>
          {asset?.kind === 'shader' && asset.animated ? (
            <span className="control-chip">
              <FiZap className="size-3" /> animated
            </span>
          ) : null}
          {asset?.kind === 'image' && asset.image_fit ? (
            <span className="control-chip">{asset.image_fit}</span>
          ) : null}
          {asset?.source_kind === 'wallpaper_engine_import' ? (
            <span className="control-chip">workshop import</span>
          ) : null}
          {asset ? <span className="control-chip">{asset.compatibility.status}</span> : null}
        </div>
      </div>
    </div>
  )
}

function InspectorMiniCard({
  icon,
  title,
  emphasis,
  meta,
}: {
  icon: React.ReactNode
  title: string
  emphasis: string
  meta: Array<string | null>
}) {
  return (
    <div className="rounded-[22px] border border-white/10 bg-white/[0.035] p-4">
      <div className="mb-2 flex items-center gap-3">
        <div className="rounded-2xl border border-white/10 bg-white/5 p-2 text-slate-200">{icon}</div>
        <div className="text-sm font-medium text-white">{title}</div>
      </div>
      <div className="text-sm font-medium text-white">{emphasis}</div>
      <div className="mt-2 space-y-1.5 text-sm text-slate-400">
        {meta.filter(Boolean).map((line) => (
          <div key={line}>{line}</div>
        ))}
      </div>
    </div>
  )
}

function PolicyToggle({
  defaultChecked,
  label,
  name,
}: {
  defaultChecked: boolean
  label: string
  name: string
}) {
  return (
    <label className="flex items-center justify-between gap-3 rounded-[22px] border border-white/10 bg-white/[0.03] px-4 py-3">
      <span className="text-sm text-slate-200">{label}</span>
      <input
        className="size-4 accent-emerald-300"
        defaultChecked={defaultChecked}
        name={name}
        type="checkbox"
      />
    </label>
  )
}

function ThemePreferenceControl({
  activeTheme,
  onChange,
  preference,
}: {
  activeTheme: ActiveTheme
  onChange: (value: ThemePreference) => void
  preference: ThemePreference
}) {
  const options: ThemePreference[] = ['system', 'light', 'dark']

  return (
    <div className="rounded-[22px] border border-white/10 bg-white/[0.03] p-4">
      <div className="mb-3 flex items-center justify-between gap-3">
        <div>
          <div className="text-sm font-medium text-white">Appearance</div>
          <div className="mt-1 text-xs text-slate-400">
            Follow the system theme or force a manual light/dark override.
          </div>
        </div>
        <span className="control-chip">
          {activeTheme}
        </span>
      </div>
      <div className="grid grid-cols-3 gap-2">
        {options.map((option) => {
          const active = preference === option
          return (
            <button
              key={option}
              type="button"
              onClick={() => onChange(option)}
              className={[
                'rounded-[16px] border px-3 py-2 text-sm capitalize transition',
                active
                  ? 'border-emerald-300/40 bg-emerald-300/15 text-emerald-100'
                  : 'border-white/10 bg-white/[0.03] text-slate-300 hover:bg-white/[0.06]',
              ].join(' ')}
            >
              {option}
            </button>
          )
        })}
      </div>
    </div>
  )
}

function EmptyState({ text }: { text: string }) {
  return (
    <div className="rounded-[22px] border border-dashed border-white/10 bg-white/[0.02] p-5 text-sm text-slate-400">
      {text}
    </div>
  )
}

function FilterChip({
  active,
  label,
  onClick,
}: {
  active: boolean
  label: string
  onClick: () => void
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={[
        'rounded-full border px-3 py-1.5 text-xs uppercase tracking-[0.16em] transition',
        active
          ? 'border-emerald-300/40 bg-emerald-300/15 text-emerald-100'
          : 'border-white/10 bg-white/[0.03] text-slate-300 hover:bg-white/[0.06]',
      ].join(' ')}
    >
      {label}
    </button>
  )
}

function ImageFitControl({
  busy,
  currentFit,
  onChange,
}: {
  busy: boolean
  currentFit: ImageFitMode
  onChange: (fit: ImageFitMode) => void
}) {
  const options: ImageFitMode[] = ['cover', 'contain', 'stretch', 'center']

  return (
    <div className="rounded-[24px] border border-white/10 bg-white/[0.04] p-4">
      <div className="mb-1 flex items-center gap-3">
        <div className="rounded-2xl border border-white/10 bg-white/5 p-2 text-slate-200">
          <FiDisc className="size-4" />
        </div>
        <div>
          <div className="text-sm font-medium text-white">Image Fit</div>
          <div className="text-xs text-slate-400">Adjust how the selected image wallpaper fills the screen.</div>
        </div>
      </div>
      <div className="mt-3 grid grid-cols-2 gap-2">
        {options.map((fit) => {
          const active = currentFit === fit
          return (
            <button
              key={fit}
              type="button"
              disabled={busy}
              onClick={() => onChange(fit)}
              className={[
                'rounded-[16px] border px-3 py-2 text-sm capitalize transition',
                active
                  ? 'border-emerald-300/40 bg-emerald-300/15 text-emerald-100'
                  : 'border-white/10 bg-white/[0.03] text-slate-300 hover:bg-white/[0.06]',
                busy ? 'cursor-not-allowed opacity-60' : '',
              ].join(' ')}
            >
              {fit}
            </button>
          )
        })}
      </div>
    </div>
  )
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
    return () => {
      observer.disconnect()
    }
  }, [])

  useEffect(() => {
    let cancelled = false

    if (!visible) {
      return
    }

    if (!previewPath) {
      setPreviewUrl(null)
      return
    }

    void loadAssetPreviewDataUrl(previewPath)
      .then((url) => {
        if (!cancelled) {
          setPreviewUrl(url)
        }
      })
      .catch(() => {
        if (!cancelled) {
          setPreviewUrl(null)
        }
      })

    return () => {
      cancelled = true
    }
  }, [previewPath, visible])

  return (
    <div
      ref={containerRef}
      className="h-full w-full bg-cover bg-center"
      style={{ backgroundImage: `url("${previewUrl ?? assetPreviewDataUri(asset)}")` }}
    />
  )
})

const AssetCard = memo(function AssetCard({
  asset,
  selected,
  workshopEnabled,
}: {
  asset: AssetMetadata
  selected: boolean
  workshopEnabled: boolean
}) {
  const detailChips = []

  if (asset.kind === 'shader' && asset.animated) {
    detailChips.push(
      <span key="animated" className="control-chip">
        <FiZap className="size-3" /> animated
      </span>,
    )
  }

  if (asset.kind === 'image' && asset.image_fit) {
    detailChips.push(
      <span key="fit" className="control-chip">
        {asset.image_fit}
      </span>,
    )
  }

  if (workshopEnabled && asset.source_kind === 'wallpaper_engine_import') {
    detailChips.push(
      <span key="source" className="control-chip">
        workshop
      </span>,
    )
  }

  return (
    <div
      className={[
        'group grid h-full min-h-[280px] grid-rows-[168px_minmax(0,1fr)] overflow-hidden border transition',
        selected
          ? 'border-emerald-300/40 bg-emerald-300/[0.08] shadow-[0_0_0_1px_rgba(89,189,167,0.2)]'
          : 'border-white/10 bg-white/[0.03] hover:border-white/20 hover:bg-white/[0.05]',
      ].join(' ')}
    >
      <div className="relative border-b border-white/10 bg-slate-950/80">
        <AssetPreview asset={asset} />
        <div className="pointer-events-none absolute inset-x-0 bottom-0 h-20 bg-gradient-to-t from-black/40 to-transparent" />
        <div className="absolute inset-x-0 top-0 flex items-start justify-between gap-3 p-3">
          <span className="preview-badge">
            {asset.source_kind === 'wallpaper_engine_import' ? 'Workshop' : asset.kind}
          </span>
          <span
            className={[
              'control-chip',
              asset.compatibility.status === 'supported'
                ? 'border-emerald-300/30 text-emerald-200'
                : asset.compatibility.status === 'partial'
                  ? 'border-amber-300/30 text-amber-200'
                  : 'border-rose-300/30 text-rose-100',
            ].join(' ')}
          >
            {asset.compatibility.status}
          </span>
        </div>
      </div>

      <div className="flex min-h-0 flex-col justify-between gap-4 p-4">
        <div className="space-y-2">
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <div className="truncate text-[15px] font-semibold text-white">{asset.name}</div>
              <div className="mt-1 truncate text-xs text-slate-400">{asset.id}</div>
            </div>
            {selected ? (
              <span className="control-chip border-emerald-300/30 text-emerald-200">
                selected
              </span>
            ) : null}
          </div>
          <p className="line-clamp-3 text-sm leading-6 text-slate-400">
            {assetSummary(asset)}
          </p>
        </div>

        <div className="flex flex-wrap gap-2">
          {detailChips}
        </div>
      </div>
    </div>
  )
})

function pointInRect(x: number, y: number, rect: { x: number; y: number; width: number; height: number }) {
  return x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height
}

function distanceBetween(ax: number, ay: number, bx: number, by: number) {
  return Math.hypot(ax - bx, ay - by)
}

function resolveEmitterViewportOverlay(
  emitter: SceneEmitterNode,
  width: number,
  _height: number,
) {
  const originX = resolveEmitterOriginX(emitter)
  const originY = resolveEmitterOriginY(emitter)
  const directionDeg = resolveEmitterDirectionDeg(emitter)
  const directionRadians = (directionDeg * Math.PI) / 180
  const directionHandleDistance = 56 / Math.max(width, 1)
  const shape = resolveEmitterShape(emitter)
  const shapeHandle = (() => {
    if (shape === 'box') {
      return {
        x: originX + (resolveEmitterRegionWidth(emitter) / 2),
        y: originY + (resolveEmitterRegionHeight(emitter) / 2),
      }
    }
    if (shape === 'circle') {
      return {
        x: originX + resolveEmitterRegionRadius(emitter),
        y: originY,
      }
    }
    if (shape === 'line') {
      const halfLength = resolveEmitterLineLength(emitter) / 2
      const angle = (resolveEmitterLineAngleDeg(emitter) * Math.PI) / 180
      return {
        x: originX + (Math.cos(angle) * halfLength),
        y: originY + (Math.sin(angle) * halfLength),
      }
    }
    return { x: originX, y: originY }
  })()

  return {
    shape,
    originX,
    originY,
    directionDeg,
    directionHandleX: originX + (Math.cos(directionRadians) * directionHandleDistance),
    directionHandleY: originY + (Math.sin(directionRadians) * directionHandleDistance),
    shapeHandleX: shapeHandle.x,
    shapeHandleY: shapeHandle.y,
    regionWidth: resolveEmitterRegionWidth(emitter),
    regionHeight: resolveEmitterRegionHeight(emitter),
    regionRadius: resolveEmitterRegionRadius(emitter),
    lineLength: resolveEmitterLineLength(emitter),
    lineAngleDeg: resolveEmitterLineAngleDeg(emitter),
  }
}

function resolveParticleAreaOverlay(
  area: SceneParticleAreaNode,
  width: number,
  height: number,
) {
  if ((area.shape ?? 'rect') === 'polygon') {
    return {
      kind: 'polygon' as const,
      points: (area.points ?? []).map((point) => ({
        x: width * point.x,
        y: height * point.y,
        leftPercent: point.x * 100,
        topPercent: point.y * 100,
      })),
    }
  }
  return {
    kind: 'rect' as const,
    x: width * area.region.x,
    y: height * area.region.y,
    width: width * area.region.width,
    height: height * area.region.height,
    leftPercent: area.region.x * 100,
    topPercent: area.region.y * 100,
    widthPercent: area.region.width * 100,
    heightPercent: area.region.height * 100,
    handleLeftPercent: (area.region.x + area.region.width) * 100,
    handleTopPercent: (area.region.y + area.region.height) * 100,
  }
}

function resolveViewportHint(
  selectedNode: SceneNode | null,
  activeTool: ComposerViewportTool,
) {
  if (!selectedNode) {
    return null
  }
  if (selectedNode.kind === 'sprite' && activeTool === 'region') {
    return 'Drag in the viewport to draw a particle region for the selected sprite.'
  }
  if (selectedNode.kind === 'particle_area' && activeTool === 'region') {
    return 'Drag in the viewport to draw a rectangular particle area.'
  }
  if (selectedNode.kind === 'particle_area' && activeTool === 'polygon') {
    return 'Click to add polygon points, or drag existing points directly in the viewport.'
  }
  if (selectedNode.kind === 'emitter' && activeTool === 'rotate') {
    return 'Drag the direction handle to aim the emitter.'
  }
  if (selectedNode.kind === 'emitter' && activeTool === 'scale') {
    return 'Drag the shape handle to resize the emitter region.'
  }
  return null
}

function resolveViewportSelection(
  nodes: SceneNode[],
  images: Map<string, HTMLImageElement>,
  previewSize: { width: number; height: number },
  canvasX: number,
  canvasY: number,
) {
  const ordered = [...nodes].reverse()
  for (const node of ordered) {
    if (!node.enabled) {
      continue
    }
    if (node.kind === 'sprite') {
      const image = images.get(node.image_key)
      if (!image) {
        continue
      }
      const layout = resolveComposerSpriteLayoutFromBounds(previewSize.width, previewSize.height, image, node, 0)
      if (pointInRect(canvasX, canvasY, layout)) {
        return node
      }
    }
    if (node.kind === 'particle_area') {
      const overlay = resolveParticleAreaOverlay(node, previewSize.width, previewSize.height)
      if (overlay.kind === 'rect' && pointInRect(canvasX, canvasY, overlay)) {
        return node
      }
      if (overlay.kind === 'polygon' && pointInPolygon(overlay.points, canvasX, canvasY)) {
        return node
      }
    }
    if (node.kind === 'emitter') {
      const overlay = resolveEmitterViewportOverlay(node, previewSize.width, previewSize.height)
      if (distanceBetween(canvasX, canvasY, overlay.originX * previewSize.width, overlay.originY * previewSize.height) <= 14) {
        return node
      }
    }
  }
  return null
}

function resolveSelectedViewportHandle(
  selectedNode: SceneNode,
  activeTool: ComposerViewportTool,
  canvasX: number,
  canvasY: number,
  previewSize: { width: number; height: number },
  images: Map<string, HTMLImageElement>,
): PreviewInteraction | null {
  if (selectedNode.kind === 'sprite') {
    const image = images.get(selectedNode.image_key)
    if (!image) {
      return null
    }
    const layout = resolveComposerSpriteLayoutFromBounds(previewSize.width, previewSize.height, image, selectedNode, 0)
    const scaleHandleX = layout.x + layout.width
    const scaleHandleY = layout.y + layout.height
    const rotateHandleX = layout.x + (layout.width / 2)
    const rotateHandleY = layout.y - 20
    if ((activeTool === 'scale' || activeTool === 'select') && distanceBetween(canvasX, canvasY, scaleHandleX, scaleHandleY) <= 16) {
      return {
        kind: 'sprite-scale',
        nodeId: selectedNode.id,
        centerX: layout.x + (layout.width / 2),
        centerY: layout.y + (layout.height / 2),
        startDistance: distanceBetween(canvasX, canvasY, layout.x + (layout.width / 2), layout.y + (layout.height / 2)),
        startScale: selectedNode.scale,
      }
    }
    if ((activeTool === 'rotate' || activeTool === 'select') && distanceBetween(canvasX, canvasY, rotateHandleX, rotateHandleY) <= 18) {
      return {
        kind: 'sprite-rotate',
        nodeId: selectedNode.id,
        centerX: layout.x + (layout.width / 2),
        centerY: layout.y + (layout.height / 2),
        startAngle: Math.atan2(canvasY - (layout.y + (layout.height / 2)), canvasX - (layout.x + (layout.width / 2))),
        startRotationDeg: selectedNode.rotation_deg,
      }
    }
    return null
  }
  if (selectedNode.kind === 'emitter') {
    const overlay = resolveEmitterViewportOverlay(selectedNode, previewSize.width, previewSize.height)
    const originX = overlay.originX * previewSize.width
    const originY = overlay.originY * previewSize.height
    if (distanceBetween(canvasX, canvasY, overlay.directionHandleX * previewSize.width, overlay.directionHandleY * previewSize.height) <= 16) {
      return {
        kind: 'emitter-direction',
        nodeId: selectedNode.id,
        originX: overlay.originX,
        originY: overlay.originY,
      }
    }
    if (distanceBetween(canvasX, canvasY, overlay.shapeHandleX * previewSize.width, overlay.shapeHandleY * previewSize.height) <= 16) {
      return {
        kind: 'emitter-shape',
        nodeId: selectedNode.id,
        shape: overlay.shape,
        startWidth: overlay.regionWidth,
        startHeight: overlay.regionHeight,
        startRadius: overlay.regionRadius,
        startLength: overlay.lineLength,
        startAngleDeg: overlay.lineAngleDeg,
        originX: overlay.originX,
        originY: overlay.originY,
      }
    }
    if (distanceBetween(canvasX, canvasY, originX, originY) <= 16) {
      return {
        kind: 'emitter-origin',
        nodeId: selectedNode.id,
      }
    }
    return null
  }
  if (selectedNode.kind === 'particle_area') {
    if ((selectedNode.shape ?? 'rect') === 'polygon') {
      const overlay = resolveParticleAreaOverlay(selectedNode, previewSize.width, previewSize.height)
      if (overlay.kind === 'polygon') {
        const index = overlay.points.findIndex((point) => distanceBetween(canvasX, canvasY, point.x, point.y) <= 12)
        if (index >= 0) {
          return {
            kind: 'area-vertex',
            nodeId: selectedNode.id,
            vertexIndex: index,
          }
        }
      }
      return null
    }
    const overlay = resolveParticleAreaOverlay(selectedNode, previewSize.width, previewSize.height)
    if (overlay.kind === 'rect') {
      if ((activeTool === 'scale' || activeTool === 'select') && distanceBetween(canvasX, canvasY, overlay.x + overlay.width, overlay.y + overlay.height) <= 16) {
        return {
          kind: 'area-resize',
          nodeId: selectedNode.id,
          startRegion: selectedNode.region,
          anchorX: selectedNode.region.x,
          anchorY: selectedNode.region.y,
        }
      }
    }
  }
  return null
}

function viewportToolLabel(tool: ComposerViewportTool) {
  switch (tool) {
    case 'select':
      return 'Select'
    case 'move':
      return 'Move'
    case 'scale':
      return 'Scale'
    case 'rotate':
      return 'Rotate'
    case 'region':
      return 'Region'
    case 'polygon':
      return 'Polygon'
  }
}

function resolveViewportToolsForNode(node: SceneNode): ComposerViewportTool[] {
  if (node.kind === 'sprite') {
    return ['select', 'move', 'scale', 'rotate', 'region']
  }
  if (node.kind === 'emitter') {
    return ['select', 'move', 'scale', 'rotate']
  }
  if (node.kind === 'particle_area') {
    return (node.shape ?? 'rect') === 'polygon'
      ? ['select', 'polygon']
      : ['select', 'move', 'scale', 'region']
  }
  return ['select']
}

type PreviewInteraction =
  | {
      kind: 'sprite-move'
      nodeId: string
      startNormX: number
      startNormY: number
      startX: number
      startY: number
    }
  | {
      kind: 'sprite-scale'
      nodeId: string
      centerX: number
      centerY: number
      startDistance: number
      startScale: number
    }
  | {
      kind: 'sprite-rotate'
      nodeId: string
      centerX: number
      centerY: number
      startAngle: number
      startRotationDeg: number
    }
  | {
      kind: 'sprite-region'
      nodeId: string
      startNormX: number
      startNormY: number
    }
  | {
      kind: 'emitter-origin'
      nodeId: string
    }
  | {
      kind: 'emitter-direction'
      nodeId: string
      originX: number
      originY: number
    }
  | {
      kind: 'emitter-shape'
      nodeId: string
      shape: SceneEmitterShape
      startWidth: number
      startHeight: number
      startRadius: number
      startLength: number
      startAngleDeg: number
      originX: number
      originY: number
    }
  | {
      kind: 'area-move'
      nodeId: string
      startNormX: number
      startNormY: number
      startRegion: SceneNormalizedRect
    }
  | {
      kind: 'area-resize'
      nodeId: string
      startRegion: SceneNormalizedRect
      anchorX: number
      anchorY: number
    }
  | {
      kind: 'area-draw'
      nodeId: string
      startNormX: number
      startNormY: number
    }
  | {
      kind: 'area-vertex'
      nodeId: string
      vertexIndex: number
    }

function ComposerEnginePreview({
  asset,
  activeTool,
  documentVisible,
  extraImages,
  onChangeNode,
  onSelectNode,
  paused,
  previewTargetSize,
  selectedNode,
  uploadedImagePath,
  uploadedImageUrl,
  nodes,
  saving,
}: {
  asset: AssetMetadata | null
  activeTool: ComposerViewportTool
  documentVisible: boolean
  extraImages: ComposerUploadImage[]
  onChangeNode: (node: SceneNode) => void
  onSelectNode: (nodeId: string) => void
  paused: boolean
  previewTargetSize: { width: number; height: number; label: string } | null
  selectedNode: SceneNode | null
  uploadedImagePath: string | null
  uploadedImageUrl: string | null
  nodes: SceneNode[]
  saving: boolean
}) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null)
  const viewportRef = useRef<HTMLDivElement | null>(null)
  const loadedImagesRef = useRef<Map<string, HTMLImageElement>>(new Map())
  const interactionRef = useRef<PreviewInteraction | null>(null)
  const nodesRef = useRef(nodes)
  const selectedNodeRef = useRef(selectedNode)
  const dragParticleRegionRef = useRef<SceneNormalizedRect | null>(null)
  const [previewUrl, setPreviewUrl] = useState<string | null>(null)
  const [previewSize, setPreviewSize] = useState({ width: 1280, height: 720 })
  const [loading, setLoading] = useState(true)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [dragParticleRegion, setDragParticleRegion] = useState<SceneNormalizedRect | null>(null)
  const [interactionActive, setInteractionActive] = useState(false)
  const previewPath = uploadedImageUrl
    ? null
    : asset?.preview_image ?? (asset?.kind === 'image' ? asset.entrypoint : null)

  useEffect(() => {
    let cancelled = false

    if (uploadedImageUrl) {
      setPreviewUrl(uploadedImageUrl)
      setLoadError(null)
      return
    }

    if (!previewPath) {
      setPreviewUrl(null)
      setLoading(false)
      setLoadError('No preview image is available for this asset.')
      return
    }

    setLoading(true)
    setLoadError(null)
    void loadAssetPreviewDataUrl(previewPath)
      .then((url) => {
        if (!cancelled) {
          setPreviewUrl(url)
        }
      })
      .catch((error) => {
        if (!cancelled) {
          setPreviewUrl(null)
          setLoading(false)
          setLoadError(error instanceof Error ? error.message : 'Failed to load preview image.')
        }
      })

    return () => {
      cancelled = true
    }
  }, [previewPath, uploadedImageUrl])

  useEffect(() => {
    nodesRef.current = nodes
  }, [nodes])

  useEffect(() => {
    selectedNodeRef.current = selectedNode
  }, [selectedNode])

  useEffect(() => {
    dragParticleRegionRef.current = dragParticleRegion
  }, [dragParticleRegion])

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) {
      return
    }

    if (!previewUrl) {
      setLoading(false)
      return
    }

    const context = canvas.getContext('2d')
    if (!context) {
      setLoading(false)
      return
    }

    let disposed = false
    let frameHandle = 0
    const imageMap = new Map<string, HTMLImageElement>()
    setLoading(true)
    setLoadError(null)
    const loadImage = (key: string, src: string, fallbackPath?: string | null) =>
      new Promise<void>((resolve, reject) => {
        const image = new Image()
        image.onload = () => {
          imageMap.set(key, image)
          resolve()
        }
        image.onerror = () => {
          if (!fallbackPath) {
            reject(new Error(`Failed to load image ${key}`))
            return
          }
          void loadAssetPreviewDataUrl(fallbackPath)
            .then((fallbackUrl) => {
              if (!fallbackUrl) {
                reject(new Error(`Failed to load image ${key}`))
                return
              }
              const fallbackImage = new Image()
              fallbackImage.onload = () => {
                imageMap.set(key, fallbackImage)
                resolve()
              }
              fallbackImage.onerror = () => reject(new Error(`Failed to decode image ${key}`))
              fallbackImage.src = fallbackUrl
            })
            .catch(() => reject(new Error(`Failed to load image ${key}`)))
        }
        image.src = src
      })

    Promise.all([
      loadImage('base', previewUrl, uploadedImagePath),
      ...extraImages.map((image) => loadImage(image.key, image.sourceUrl, image.path)),
      ])
      .then(() => {
        if (disposed) {
          return
        }
        loadedImagesRef.current = imageMap
        setLoading(false)
        const startedAt = performance.now()
        let lastRenderedAt = 0
        let lastSizeKey = ''
        const render = (now: number) => {
          if (disposed) {
            return
          }
          if (!documentVisible || paused) {
            frameHandle = window.requestAnimationFrame(render)
            return
          }
          const baseImage = imageMap.get('base')
          const logicalWidth = Math.max(1, previewTargetSize?.width ?? baseImage?.naturalWidth ?? 1280)
          const logicalHeight = Math.max(1, previewTargetSize?.height ?? baseImage?.naturalHeight ?? 720)
          const maxLongEdge = interactionActive ? 960 : 1280
          const scale = Math.min(1, maxLongEdge / Math.max(logicalWidth, logicalHeight))
          const targetWidth = Math.max(1, Math.round(logicalWidth * scale))
          const targetHeight = Math.max(1, Math.round(logicalHeight * scale))
          if (canvas.width !== targetWidth || canvas.height !== targetHeight) {
            canvas.width = targetWidth
            canvas.height = targetHeight
          }
          const sizeKey = `${targetWidth}x${targetHeight}`
          if (sizeKey !== lastSizeKey) {
            lastSizeKey = sizeKey
            setPreviewSize({ width: targetWidth, height: targetHeight })
          }
          const frameIntervalMs = 1000 / (interactionActive ? 20 : 24)
          if (now - lastRenderedAt >= frameIntervalMs) {
            drawComposerSceneFrame(
              context,
              imageMap,
              nodesRef.current,
              selectedNodeRef.current,
              dragParticleRegionRef.current,
              (now - startedAt) / 1000,
            )
            lastRenderedAt = now
          }
          frameHandle = window.requestAnimationFrame(render)
        }
        render(startedAt)
      })
      .catch(() => {
        if (!disposed) {
          setLoading(false)
          setLoadError('Failed to decode scene source image for preview.')
        }
      })

    return () => {
      disposed = true
      loadedImagesRef.current = new Map()
      window.cancelAnimationFrame(frameHandle)
    }
  }, [
    documentVisible,
    extraImages,
    interactionActive,
    paused,
    previewTargetSize?.height,
    previewTargetSize?.width,
    uploadedImagePath,
    previewUrl,
  ])

  useEffect(() => {
    if (activeTool !== 'region') {
      setDragParticleRegion(null)
    }
  }, [activeTool])

  const selectedSpriteLayout = (() => {
    if (!selectedNode || selectedNode.kind !== 'sprite') {
      return null
    }
    const image = loadedImagesRef.current.get(selectedNode.image_key)
    if (!image) {
      return null
    }
    return resolveComposerSpriteLayoutFromBounds(previewSize.width, previewSize.height, image, selectedNode, 0)
  })()
  const selectedEmitterOverlay = selectedNode?.kind === 'emitter'
    ? resolveEmitterViewportOverlay(selectedNode, previewSize.width, previewSize.height)
    : null
  const selectedParticleAreaOverlay = selectedNode?.kind === 'particle_area'
    ? resolveParticleAreaOverlay(selectedNode, previewSize.width, previewSize.height)
    : null
  const toolHint = resolveViewportHint(selectedNode, activeTool)

  const updateSelectedNode = (updater: (node: SceneNode) => SceneNode) => {
    if (!selectedNode) {
      return
    }
    onChangeNode(updater(selectedNode))
  }

  const pointerToPreview = (event: React.PointerEvent<HTMLDivElement>) => {
    const target = event.currentTarget
    const rect = target.getBoundingClientRect()
    const x = clamp01((event.clientX - rect.left) / Math.max(rect.width, 1))
    const y = clamp01((event.clientY - rect.top) / Math.max(rect.height, 1))
    return {
      normalizedX: x,
      normalizedY: y,
      canvasX: x * previewSize.width,
      canvasY: y * previewSize.height,
      rect,
    }
  }

  const commitDragRegion = () => {
    if (!selectedNode || activeTool !== 'region' || !dragParticleRegion) {
      return
    }
    if (selectedNode.kind === 'sprite') {
      onChangeNode({ ...selectedNode, particle_region: dragParticleRegion })
    }
    if (selectedNode.kind === 'particle_area' && (selectedNode.shape ?? 'rect') === 'rect') {
      onChangeNode({ ...selectedNode, region: dragParticleRegion })
    }
  }

  return (
    <div className="flex h-full min-h-[360px] items-center justify-center">
      <div
        ref={viewportRef}
        className={[
          'relative w-full overflow-hidden border border-white/10 bg-slate-950/90',
          selectedNode ? 'cursor-default' : '',
        ].join(' ')}
        onPointerDown={(event) => {
          const pointer = pointerToPreview(event)
          if (!selectedNode) {
            return
          }

          if (selectedNode.kind === 'particle_area' && (selectedNode.shape ?? 'rect') === 'polygon' && activeTool === 'polygon') {
            const points = [...(selectedNode.points ?? []), { x: pointer.normalizedX, y: pointer.normalizedY }]
            onChangeNode({
              ...selectedNode,
              points,
              region: polygonBounds(points) ?? selectedNode.region,
            })
            return
          }

          const selectedHandle = resolveSelectedViewportHandle(selectedNode, activeTool, pointer.canvasX, pointer.canvasY, previewSize, loadedImagesRef.current)
          if (selectedHandle) {
            event.currentTarget.setPointerCapture(event.pointerId)
            interactionRef.current = selectedHandle
            setInteractionActive(true)
            return
          }

          const hitNode = resolveViewportSelection(nodes, loadedImagesRef.current, previewSize, pointer.canvasX, pointer.canvasY)
          if (hitNode && hitNode.id !== selectedNode.id) {
            onSelectNode(hitNode.id)
            return
          }

          if (selectedNode.kind === 'sprite' && selectedSpriteLayout) {
            if (activeTool === 'region') {
              interactionRef.current = {
                kind: 'sprite-region',
                nodeId: selectedNode.id,
                startNormX: pointer.normalizedX,
                startNormY: pointer.normalizedY,
              }
              setDragParticleRegion({ x: pointer.normalizedX, y: pointer.normalizedY, width: 0.001, height: 0.001 })
              event.currentTarget.setPointerCapture(event.pointerId)
              setInteractionActive(true)
              return
            }
            if (pointInRect(pointer.canvasX, pointer.canvasY, selectedSpriteLayout)) {
              interactionRef.current = {
                kind: 'sprite-move',
                nodeId: selectedNode.id,
                startNormX: pointer.normalizedX,
                startNormY: pointer.normalizedY,
                startX: selectedNode.x,
                startY: selectedNode.y,
              }
              event.currentTarget.setPointerCapture(event.pointerId)
              setInteractionActive(true)
              return
            }
          }

          if (selectedNode.kind === 'emitter') {
            interactionRef.current = { kind: 'emitter-origin', nodeId: selectedNode.id }
            updateSelectedNode((node) => node.kind === 'emitter' ? { ...node, origin_x: pointer.normalizedX, origin_y: pointer.normalizedY } : node)
            event.currentTarget.setPointerCapture(event.pointerId)
            setInteractionActive(true)
            return
          }

          if (
            selectedNode.kind === 'particle_area'
            && (selectedNode.shape ?? 'rect') === 'rect'
            && selectedParticleAreaOverlay
            && selectedParticleAreaOverlay.kind === 'rect'
          ) {
            if (activeTool === 'region') {
              interactionRef.current = {
                kind: 'area-draw',
                nodeId: selectedNode.id,
                startNormX: pointer.normalizedX,
                startNormY: pointer.normalizedY,
              }
              setDragParticleRegion({ x: pointer.normalizedX, y: pointer.normalizedY, width: 0.001, height: 0.001 })
              event.currentTarget.setPointerCapture(event.pointerId)
              setInteractionActive(true)
              return
            }
            if (pointInRect(pointer.canvasX, pointer.canvasY, selectedParticleAreaOverlay)) {
              interactionRef.current = {
                kind: 'area-move',
                nodeId: selectedNode.id,
                startNormX: pointer.normalizedX,
                startNormY: pointer.normalizedY,
                startRegion: selectedNode.region,
              }
              event.currentTarget.setPointerCapture(event.pointerId)
              setInteractionActive(true)
            }
          }
        }}
        onPointerMove={(event) => {
          const interaction = interactionRef.current
          if (!interaction || !selectedNode || (event.buttons & 1) === 0) {
            return
          }
          const pointer = pointerToPreview(event)
          if (interaction.kind === 'sprite-region' || interaction.kind === 'area-draw') {
            setDragParticleRegion(normalizeDraggedRect(interaction.startNormX, interaction.startNormY, pointer.normalizedX, pointer.normalizedY))
            return
          }
          if (selectedNode.id !== interaction.nodeId) {
            return
          }
          if (selectedNode.kind === 'sprite') {
            if (interaction.kind === 'sprite-move') {
              onChangeNode({
                ...selectedNode,
                x: interaction.startX + ((pointer.normalizedX - interaction.startNormX) * previewSize.width),
                y: interaction.startY + ((pointer.normalizedY - interaction.startNormY) * previewSize.height),
              })
              return
            }
            if (interaction.kind === 'sprite-scale') {
              const distance = distanceBetween(pointer.canvasX, pointer.canvasY, interaction.centerX, interaction.centerY)
              const scale = Math.max(0.1, interaction.startScale * (distance / Math.max(interaction.startDistance, 1)))
              onChangeNode({ ...selectedNode, scale })
              return
            }
            if (interaction.kind === 'sprite-rotate') {
              const angle = Math.atan2(pointer.canvasY - interaction.centerY, pointer.canvasX - interaction.centerX)
              onChangeNode({
                ...selectedNode,
                rotation_deg: interaction.startRotationDeg + (((angle - interaction.startAngle) * 180) / Math.PI),
              })
              return
            }
          }
          if (selectedNode.kind === 'emitter') {
            if (interaction.kind === 'emitter-origin') {
              onChangeNode({ ...selectedNode, origin_x: pointer.normalizedX, origin_y: pointer.normalizedY })
              return
            }
            if (interaction.kind === 'emitter-direction') {
              const angle = Math.atan2(pointer.normalizedY - interaction.originY, pointer.normalizedX - interaction.originX)
              onChangeNode({ ...selectedNode, direction_deg: (angle * 180) / Math.PI })
              return
            }
            if (interaction.kind === 'emitter-shape') {
              const deltaX = pointer.normalizedX - interaction.originX
              const deltaY = pointer.normalizedY - interaction.originY
              if (interaction.shape === 'box') {
                onChangeNode({
                  ...selectedNode,
                  region_width: Math.max(0.02, Math.abs(deltaX) * 2),
                  region_height: Math.max(0.02, Math.abs(deltaY) * 2),
                })
                return
              }
              if (interaction.shape === 'circle') {
                onChangeNode({
                  ...selectedNode,
                  region_radius: Math.max(0.01, distanceBetween(pointer.normalizedX, pointer.normalizedY, interaction.originX, interaction.originY)),
                })
                return
              }
              if (interaction.shape === 'line') {
                onChangeNode({
                  ...selectedNode,
                  line_length: Math.max(0.02, distanceBetween(pointer.normalizedX, pointer.normalizedY, interaction.originX, interaction.originY) * 2),
                  line_angle_deg: (Math.atan2(deltaY, deltaX) * 180) / Math.PI,
                })
              }
            }
          }
          if (selectedNode.kind === 'particle_area') {
            if (interaction.kind === 'area-move') {
              const nextX = clamp01(interaction.startRegion.x + (pointer.normalizedX - interaction.startNormX))
              const nextY = clamp01(interaction.startRegion.y + (pointer.normalizedY - interaction.startNormY))
              onChangeNode({
                ...selectedNode,
                region: {
                  ...interaction.startRegion,
                  x: Math.min(nextX, 1 - interaction.startRegion.width),
                  y: Math.min(nextY, 1 - interaction.startRegion.height),
                },
              })
              return
            }
            if (interaction.kind === 'area-resize') {
              onChangeNode({
                ...selectedNode,
                region: normalizeDraggedRect(interaction.anchorX, interaction.anchorY, pointer.normalizedX, pointer.normalizedY),
              })
              return
            }
            if (interaction.kind === 'area-vertex') {
              const points = [...(selectedNode.points ?? [])]
              points[interaction.vertexIndex] = { x: pointer.normalizedX, y: pointer.normalizedY }
              onChangeNode({
                ...selectedNode,
                points,
                region: polygonBounds(points) ?? selectedNode.region,
              })
            }
          }
        }}
        onPointerUp={(event) => {
          if (event.currentTarget.hasPointerCapture(event.pointerId)) {
            event.currentTarget.releasePointerCapture(event.pointerId)
          }
          commitDragRegion()
          interactionRef.current = null
          setDragParticleRegion(null)
          setInteractionActive(false)
        }}
        onPointerLeave={() => {
          interactionRef.current = null
          setInteractionActive(false)
        }}
        style={{ aspectRatio: `${previewSize.width} / ${previewSize.height}` }}
      >
        <canvas
          ref={canvasRef}
          className="h-full w-full object-contain"
          width={previewSize.width}
          height={previewSize.height}
        />
        {selectedNode ? (
          <div className="pointer-events-none absolute left-4 top-4">
            <div className="surface-panel px-3 py-2 text-xs text-slate-200">
              <div className="font-medium text-slate-50">{selectedNode.name}</div>
              <div className="mt-1 uppercase tracking-[0.16em] text-slate-400">{selectedNode.kind} · {viewportToolLabel(activeTool)}</div>
              {previewTargetSize ? (
                <div className="mt-1 text-[11px] text-slate-500">
                  {previewTargetSize.label} · {previewSize.width}×{previewSize.height}
                </div>
              ) : null}
            </div>
          </div>
        ) : null}
        {toolHint ? (
          <div className="pointer-events-none absolute left-4 top-20">
            <div className="surface-panel px-3 py-2 text-xs text-slate-200">
              {toolHint}
            </div>
          </div>
        ) : null}
        {selectedSpriteLayout ? (
          <SpriteViewportOverlay activeTool={activeTool} layout={selectedSpriteLayout} previewSize={previewSize} />
        ) : null}
        {selectedEmitterOverlay ? (
          <EmitterViewportOverlay activeTool={activeTool} overlay={selectedEmitterOverlay} />
        ) : null}
        {selectedParticleAreaOverlay ? (
          <ParticleAreaViewportOverlay activeTool={activeTool} overlay={selectedParticleAreaOverlay} />
        ) : null}
        {loading ? (
          <div className="absolute inset-0 flex items-center justify-center bg-slate-950/55 backdrop-blur-sm">
            <div className="surface-panel flex items-center gap-3 px-4 py-3 text-sm text-slate-200">
              <FiRefreshCw className="size-4 animate-spin text-cyan-200" />
              Loading scene source
            </div>
          </div>
        ) : null}
        {loadError ? (
          <div className="absolute inset-x-4 bottom-4">
            <div className="tone-warning p-3 text-sm">{loadError}</div>
          </div>
        ) : null}
        {saving ? (
          <div className="absolute inset-x-4 bottom-4">
            <div className="surface-panel flex items-center gap-3 px-4 py-3 text-sm text-slate-100">
              <FiRefreshCw className="size-4 animate-spin text-cyan-200" />
              Packaging scene document and assets...
            </div>
          </div>
        ) : null}
      </div>
    </div>
  )
}

function EmitterViewportOverlay({
  activeTool,
  overlay,
}: {
  activeTool: ComposerViewportTool
  overlay: ReturnType<typeof resolveEmitterViewportOverlay>
}) {
  const left = `${overlay.originX * 100}%`
  const top = `${overlay.originY * 100}%`
  const shape = overlay.shape
  const common = 'pointer-events-none absolute border border-cyan-200/70 bg-cyan-200/5'
  return (
    <>
      {shape === 'box' ? (
        <div
          className={common}
          style={{
            left,
            top,
            width: `${overlay.regionWidth * 100}%`,
            height: `${overlay.regionHeight * 100}%`,
            transform: 'translate(-50%, -50%)',
          }}
        />
      ) : null}
      {shape === 'circle' ? (
        <div
          className={`${common} rounded-full`}
          style={{
            left,
            top,
            width: `${overlay.regionRadius * 200}%`,
            height: `${overlay.regionRadius * 200}%`,
            transform: 'translate(-50%, -50%)',
          }}
        />
      ) : null}
      {shape === 'line' ? (
        <div
          className="pointer-events-none absolute h-[2px] bg-cyan-200/85"
          style={{
            left,
            top,
            width: `${overlay.lineLength * 100}%`,
            transform: `translate(-50%, -50%) rotate(${overlay.lineAngleDeg}deg)`,
            transformOrigin: 'center',
          }}
        />
      ) : null}
      <div
        className="pointer-events-none absolute rounded-full border border-cyan-200/90 bg-cyan-100/10 shadow-[0_0_0_1px_rgba(7,19,28,0.4)]"
        style={{
          left,
          top,
          width: '18px',
          height: '18px',
          transform: 'translate(-50%, -50%)',
        }}
      />
      <div
        className="pointer-events-none absolute rounded-full border border-cyan-100/90 bg-cyan-100"
        style={{
          left: `${overlay.directionHandleX * 100}%`,
          top: `${overlay.directionHandleY * 100}%`,
          width: '12px',
          height: '12px',
          transform: 'translate(-50%, -50%)',
          boxShadow: activeTool === 'rotate' ? '0 0 0 4px rgba(103,232,249,0.18)' : 'none',
        }}
      />
      <div
        className="pointer-events-none absolute rounded-[3px] border border-cyan-100/90 bg-cyan-100/85"
        style={{
          left: `${overlay.shapeHandleX * 100}%`,
          top: `${overlay.shapeHandleY * 100}%`,
          width: '11px',
          height: '11px',
          transform: 'translate(-50%, -50%)',
          boxShadow: activeTool === 'scale' ? '0 0 0 4px rgba(103,232,249,0.18)' : 'none',
        }}
      />
      <div
        className="pointer-events-none absolute h-[2px] origin-left bg-cyan-200/85"
        style={{
          left,
          top,
          width: '56px',
          transform: `translateY(-50%) rotate(${overlay.directionDeg}deg)`,
        }}
      />
    </>
  )
}

function SpriteViewportOverlay({
  activeTool,
  layout,
  previewSize,
}: {
  activeTool: ComposerViewportTool
  layout: { x: number; y: number; width: number; height: number }
  previewSize: { width: number; height: number }
}) {
  return (
    <>
      <div
        className="pointer-events-none absolute border border-cyan-200/85"
        style={{
          left: `${(layout.x / Math.max(previewSize.width, 1)) * 100}%`,
          top: `${(layout.y / Math.max(previewSize.height, 1)) * 100}%`,
          width: `${(layout.width / Math.max(previewSize.width, 1)) * 100}%`,
          height: `${(layout.height / Math.max(previewSize.height, 1)) * 100}%`,
        }}
      />
      <div
        className="pointer-events-none absolute rounded-[3px] border border-cyan-100/90 bg-cyan-100/85"
        style={{
          left: `${((layout.x + layout.width) / Math.max(previewSize.width, 1)) * 100}%`,
          top: `${((layout.y + layout.height) / Math.max(previewSize.height, 1)) * 100}%`,
          width: '11px',
          height: '11px',
          transform: 'translate(-50%, -50%)',
          boxShadow: activeTool === 'scale' ? '0 0 0 4px rgba(103,232,249,0.18)' : 'none',
        }}
      />
      <div
        className="pointer-events-none absolute rounded-full border border-cyan-100/90 bg-cyan-100"
        style={{
          left: `${((layout.x + (layout.width / 2)) / Math.max(previewSize.width, 1)) * 100}%`,
          top: `${((layout.y - 20) / Math.max(previewSize.height, 1)) * 100}%`,
          width: '12px',
          height: '12px',
          transform: 'translate(-50%, -50%)',
          boxShadow: activeTool === 'rotate' ? '0 0 0 4px rgba(103,232,249,0.18)' : 'none',
        }}
      />
    </>
  )
}

function ParticleAreaViewportOverlay({
  activeTool,
  overlay,
}: {
  activeTool: ComposerViewportTool
  overlay: ReturnType<typeof resolveParticleAreaOverlay>
}) {
  if (!overlay) {
    return null
  }
  if (overlay.kind === 'rect') {
    return (
      <>
        <div
          className="pointer-events-none absolute border border-cyan-200/85 bg-cyan-200/10"
          style={{
            left: `${overlay.leftPercent}%`,
            top: `${overlay.topPercent}%`,
            width: `${overlay.widthPercent}%`,
            height: `${overlay.heightPercent}%`,
          }}
        />
        <div
          className="pointer-events-none absolute rounded-[3px] border border-cyan-100/90 bg-cyan-100/85"
          style={{
            left: `${overlay.handleLeftPercent}%`,
            top: `${overlay.handleTopPercent}%`,
            width: '11px',
            height: '11px',
            transform: 'translate(-50%, -50%)',
            boxShadow: activeTool === 'scale' ? '0 0 0 4px rgba(103,232,249,0.18)' : 'none',
          }}
        />
      </>
    )
  }
  return (
    <>
      {overlay.points.map((point, index) => (
        <div
          key={`${index}-${point.leftPercent}-${point.topPercent}`}
          className="pointer-events-none absolute rounded-full border border-cyan-100/90 bg-cyan-100"
          style={{
            left: `${point.leftPercent}%`,
            top: `${point.topPercent}%`,
            width: '12px',
            height: '12px',
            transform: 'translate(-50%, -50%)',
            boxShadow: activeTool === 'polygon' ? '0 0 0 4px rgba(103,232,249,0.18)' : 'none',
          }}
        />
      ))}
    </>
  )
}

function ComposerNodeInspector({
  availableImageKeys,
  viewportTool,
  node,
  onOpenParticleEditor,
  onActivateViewportTool,
  onChange,
}: {
  availableImageKeys: Array<{ key: string; label: string }>
  viewportTool: ComposerViewportTool
  node: SceneNode
  onOpenParticleEditor: (nodeId: string) => void
  onActivateViewportTool: (nodeId: string, tool: ComposerViewportTool) => void
  onChange: (node: SceneNode) => void
}) {
  return (
    <div className="space-y-4">
      <div className="surface-muted space-y-4 p-4">
        <label className="flex items-center justify-between gap-3 text-sm text-slate-300">
          <span>Enabled</span>
          <input
            checked={node.enabled}
            className="size-4 accent-emerald-300"
            onChange={(event) => onChange({ ...node, enabled: event.currentTarget.checked } as SceneNode)}
            type="checkbox"
          />
        </label>
        <label className="block">
          <span className="mb-2 block text-xs uppercase tracking-[0.18em] text-slate-400">Name</span>
          <input
            className="field-shell w-full"
            onChange={(event) => onChange({ ...node, name: event.currentTarget.value } as SceneNode)}
            type="text"
            value={node.name}
          />
        </label>
      </div>
      {node.kind === 'sprite' ? (
        <ComposerSpriteInspector
          availableImageKeys={availableImageKeys}
          viewportTool={viewportTool}
          node={node}
          onActivateViewportTool={onActivateViewportTool}
          onChange={onChange}
        />
      ) : null}
      {node.kind === 'effect' ? <ComposerEffectInspector node={node} onChange={onChange} /> : null}
      {node.kind === 'emitter' ? <ComposerEmitterInspector availableImageKeys={availableImageKeys} node={node} onChange={onChange} onOpenParticleEditor={onOpenParticleEditor} /> : null}
      {node.kind === 'particle_area' ? <ComposerParticleAreaInspector viewportTool={viewportTool} node={node} onActivateViewportTool={onActivateViewportTool} onChange={onChange} /> : null}
    </div>
  )
}

function ComposerParticleAreaInspector({
  viewportTool,
  node,
  onActivateViewportTool,
  onChange,
}: {
  viewportTool: ComposerViewportTool
  node: SceneParticleAreaNode
  onActivateViewportTool: (nodeId: string, tool: ComposerViewportTool) => void
  onChange: (node: SceneNode) => void
}) {
  return (
    <div className="space-y-4">
      <ComposerSection title="Region" defaultOpen>
        <div className="space-y-4">
          <div className="grid grid-cols-2 gap-2">
            {(['rect', 'polygon'] as const).map((shape) => (
              <button
                key={shape}
                className={['secondary-button justify-center capitalize', (node.shape ?? 'rect') === shape ? 'border-cyan-300/40 text-cyan-100' : ''].join(' ')}
                onClick={() => onChange({
                  ...node,
                  shape,
                  points: shape === 'polygon'
                    ? (node.points && node.points.length >= 3 ? node.points : [
                      { x: 0.25, y: 0.25 },
                      { x: 0.75, y: 0.25 },
                      { x: 0.5, y: 0.55 },
                    ])
                    : node.points,
                })}
                type="button"
              >
                {shape}
              </button>
            ))}
          </div>
          {(node.shape ?? 'rect') === 'rect' ? (
            <div className="grid gap-3 sm:grid-cols-2">
              <RangeField label="X" max={100} min={0} step={1} value={node.region.x * 100} onChange={(value) => onChange({ ...node, region: { ...node.region, x: value / 100 } })} />
              <RangeField label="Y" max={100} min={0} step={1} value={node.region.y * 100} onChange={(value) => onChange({ ...node, region: { ...node.region, y: value / 100 } })} />
              <RangeField label="Width" max={100} min={1} step={1} value={node.region.width * 100} onChange={(value) => onChange({ ...node, region: { ...node.region, width: value / 100 } })} />
              <RangeField label="Height" max={100} min={1} step={1} value={node.region.height * 100} onChange={(value) => onChange({ ...node, region: { ...node.region, height: value / 100 } })} />
            </div>
          ) : (
            <div className="space-y-3">
              <div className="surface-muted p-3 text-sm text-slate-300">
                Click in the viewport to add polygon vertices. The area closes automatically from the last point back to the first.
              </div>
              <div className="grid gap-2">
                {(node.points ?? []).map((point, index) => (
                  <div key={`${index}-${point.x}-${point.y}`} className="surface-muted flex items-center justify-between gap-3 px-3 py-2 text-sm text-slate-300">
                    <span>Point {index + 1}: {(point.x * 100).toFixed(0)}%, {(point.y * 100).toFixed(0)}%</span>
                    <button
                      className="icon-button"
                      onClick={() => onChange({ ...node, points: (node.points ?? []).filter((_, pointIndex) => pointIndex !== index) })}
                      type="button"
                    >
                      <FiTrash2 className="size-4" />
                    </button>
                  </div>
                ))}
              </div>
              <button
                className="secondary-button w-full justify-center"
                onClick={() => onChange({ ...node, points: [] })}
                type="button"
              >
                Clear polygon
              </button>
            </div>
          )}
          <button
            className={[
              'secondary-button w-full justify-center',
              viewportTool === ((node.shape ?? 'rect') === 'polygon' ? 'polygon' : 'region') ? 'border-cyan-300/40 text-cyan-100' : '',
            ].join(' ')}
            onClick={() => onActivateViewportTool(node.id, (node.shape ?? 'rect') === 'polygon' ? 'polygon' : 'region')}
            type="button"
          >
            {(node.shape ?? 'rect') === 'polygon' ? 'Edit polygon points in viewport' : 'Draw area in viewport'}
          </button>
        </div>
      </ComposerSection>
      <ComposerSection title="Behavior" defaultOpen>
        <div className="space-y-4">
          <label className="flex items-center justify-between gap-3 text-sm text-slate-300">
            <span>Occluder</span>
            <input
              checked={node.occluder}
              className="size-4 accent-emerald-300"
              onChange={(event) => onChange({ ...node, occluder: event.currentTarget.checked })}
              type="checkbox"
            />
          </label>
          <label className="flex items-center justify-between gap-3 text-sm text-slate-300">
            <span>Landing surface</span>
            <input
              checked={node.surface}
              className="size-4 accent-emerald-300"
              onChange={(event) => onChange({ ...node, surface: event.currentTarget.checked })}
              type="checkbox"
            />
          </label>
        </div>
      </ComposerSection>
    </div>
  )
}

function ComposerSpriteInspector({
  availableImageKeys,
  viewportTool,
  node,
  onActivateViewportTool,
  onChange,
}: {
  availableImageKeys: Array<{ key: string; label: string }>
  viewportTool: ComposerViewportTool
  node: SceneSpriteNode
  onActivateViewportTool: (nodeId: string, tool: ComposerViewportTool) => void
  onChange: (node: SceneNode) => void
}) {
  const isBaseSprite = node.image_key === 'base' || node.id === 'sprite-base'
  const behaviorKinds: SceneBehaviorKind[] = ['drift', 'pulse', 'orbit']
  const activeBehaviorKinds = new Set(node.behaviors.map((behavior) => behavior.kind))
  return (
    <div className="space-y-4">
      <ComposerSection title="Basic" defaultOpen>
        <div className="grid gap-3 sm:grid-cols-2">
          <RangeField label="Opacity" max={1} min={0} step={0.01} value={node.opacity} onChange={(value) => onChange({ ...node, opacity: value })} />
          <RangeField label="Scale" max={2.5} min={0.25} step={0.01} value={node.scale} onChange={(value) => onChange({ ...node, scale: value })} />
          <RangeField label="Offset X" max={400} min={-400} step={1} value={node.x} onChange={(value) => onChange({ ...node, x: value })} />
          <RangeField label="Offset Y" max={400} min={-400} step={1} value={node.y} onChange={(value) => onChange({ ...node, y: value })} />
        </div>
      </ComposerSection>
      <ComposerSection title="Appearance" defaultOpen>
        <div className="space-y-4">
          <div>
            <div className="mb-2 text-xs uppercase tracking-[0.18em] text-slate-400">Image Source</div>
            <div className="grid gap-2">
              {availableImageKeys.map((image) => (
                <button
                  key={image.key}
                  className={['secondary-button justify-center', node.image_key === image.key ? 'border-cyan-300/40 text-cyan-100' : ''].join(' ')}
                  onClick={() => onChange({ ...node, image_key: image.key })}
                  type="button"
                >
                  {image.label}
                </button>
              ))}
            </div>
          </div>
          <div>
            <div className="mb-2 text-xs uppercase tracking-[0.18em] text-slate-400">Fit</div>
            {isBaseSprite ? (
              <div className="surface-muted p-3 text-sm text-slate-300">
                Base image is fixed to <span className="font-medium text-slate-100">cover</span> so the scene fills the screen.
              </div>
            ) : (
              <div className="grid grid-cols-2 gap-2">
                {(['cover', 'contain', 'stretch', 'center'] as const).map((fit) => (
                  <button
                    key={fit}
                    className={['secondary-button justify-center', node.fit === fit ? 'border-cyan-300/40 text-cyan-100' : ''].join(' ')}
                    onClick={() => onChange({ ...node, fit })}
                    type="button"
                  >
                    {fit}
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>
      </ComposerSection>
      <ComposerSection title="Motion" defaultOpen={node.behaviors.length > 0}>
        <div className="space-y-4">
          <div>
            <div className="mb-2 text-xs uppercase tracking-[0.18em] text-slate-400">Behaviors</div>
            <div className="grid grid-cols-3 gap-2">
              {behaviorKinds.map((kind) => (
                <button
                  key={kind}
                  className={['secondary-button justify-center capitalize', activeBehaviorKinds.has(kind) ? 'border-cyan-300/40 text-cyan-100' : ''].join(' ')}
                  onClick={() => onChange(toggleSpriteBehavior(node, kind))}
                  type="button"
                >
                  {kind}
                </button>
              ))}
            </div>
          </div>
          {node.behaviors.map((behavior) => (
            <div key={behavior.kind} className="surface-muted space-y-3 p-3">
              <div className="text-xs uppercase tracking-[0.16em] text-slate-400">{behavior.kind}</div>
              <RangeField label="Speed" max={4} min={0} step={0.05} value={behavior.speed} onChange={(value) => onChange(updateSpriteBehavior(node, behavior.kind, { speed: value }))} />
              <RangeField label="Amount X" max={240} min={-240} step={1} value={behavior.amount_x} onChange={(value) => onChange(updateSpriteBehavior(node, behavior.kind, { amount_x: value }))} />
              <RangeField label="Amount Y" max={240} min={-240} step={1} value={behavior.amount_y} onChange={(value) => onChange(updateSpriteBehavior(node, behavior.kind, { amount_y: value }))} />
              <RangeField label="Amount" max={240} min={-240} step={1} value={behavior.amount} onChange={(value) => onChange(updateSpriteBehavior(node, behavior.kind, { amount: value }))} />
            </div>
          ))}
        </div>
      </ComposerSection>
      <ComposerSection title="Particles">
        <div className="space-y-4">
          <label className="flex items-center justify-between gap-3 text-sm text-slate-300">
            <span>Occlude particles</span>
            <input
              checked={Boolean(node.particle_occluder)}
              className="size-4 accent-emerald-300"
              onChange={(event) => onChange({ ...node, particle_occluder: event.currentTarget.checked })}
              type="checkbox"
            />
          </label>
          <label className="flex items-center justify-between gap-3 text-sm text-slate-300">
            <span>Landing surface</span>
            <input
              checked={Boolean(node.particle_surface)}
              className="size-4 accent-emerald-300"
              onChange={(event) => onChange({ ...node, particle_surface: event.currentTarget.checked })}
              type="checkbox"
            />
          </label>
          <div className="surface-muted p-3 text-sm text-slate-300">
            Mark foreground sprites as particle occluders to hide rain or sparks behind them. Mark ledges or ground sprites as landing surfaces so snow and dust can settle on their top edge.
          </div>
          {node.particle_occluder || node.particle_surface ? (
            <div className="grid gap-2 sm:grid-cols-2">
              <button
                className={['secondary-button justify-center', viewportTool === 'region' ? 'border-cyan-300/40 text-cyan-100' : ''].join(' ')}
                onClick={() => onActivateViewportTool(node.id, 'region')}
                type="button"
              >
                Edit region in viewport
              </button>
              <button
                className="secondary-button justify-center"
                onClick={() => onChange({ ...node, particle_region: null })}
                type="button"
              >
                Clear custom region
              </button>
            </div>
          ) : null}
          {node.particle_region ? (
            <div className="surface-muted p-3 text-xs text-slate-300">
              Region: x {(node.particle_region.x * 100).toFixed(0)}%, y {(node.particle_region.y * 100).toFixed(0)}%, w {(node.particle_region.width * 100).toFixed(0)}%, h {(node.particle_region.height * 100).toFixed(0)}%
            </div>
          ) : null}
        </div>
      </ComposerSection>
    </div>
  )
}

function ComposerEffectInspector({
  node,
  onChange,
}: {
  node: Extract<SceneNode, { kind: 'effect' }>
  onChange: (node: SceneNode) => void
}) {
  const colorHex = resolveEffectColorHex(node)
  return (
    <div className="space-y-4">
      <ComposerSection title="Basic" defaultOpen>
        <div className="grid grid-cols-2 gap-2">
          {SCENE_EFFECT_OPTIONS.map((option) => (
            <button
              key={option.effect}
              className={['secondary-button justify-center', node.effect === option.effect ? 'border-cyan-300/40 text-cyan-100' : ''].join(' ')}
              onClick={() => onChange({ ...node, effect: option.effect, color_hex: defaultEffectColorHex(option.effect) })}
              type="button"
            >
              {option.label}
            </button>
          ))}
        </div>
      </ComposerSection>
      <ComposerSection title="Appearance" defaultOpen>
        <div className="space-y-4">
          <label className="block">
            <div className="mb-2 flex items-center justify-between text-xs uppercase tracking-[0.16em] text-slate-400">
              <span>Color</span>
              <span>{colorHex}</span>
            </div>
            <input
              className="h-10 w-full border border-white/10 bg-transparent p-1"
              type="color"
              value={colorHex}
              onChange={(event) => onChange({ ...node, color_hex: event.currentTarget.value })}
            />
          </label>
          <RangeField label="Opacity" max={1} min={0} step={0.01} value={node.opacity} onChange={(value) => onChange({ ...node, opacity: value })} />
          <RangeField label="Intensity" max={2.5} min={0} step={0.05} value={node.intensity} onChange={(value) => onChange({ ...node, intensity: value })} />
        </div>
      </ComposerSection>
      <ComposerSection title="Motion">
        <RangeField label="Speed" max={3} min={0} step={0.05} value={node.speed} onChange={(value) => onChange({ ...node, speed: value })} />
      </ComposerSection>
    </div>
  )
}

function ComposerEmitterInspector({
  availableImageKeys,
  node,
  onOpenParticleEditor,
  onChange,
}: {
  availableImageKeys: Array<{ key: string; label: string }>
  node: Extract<SceneNode, { kind: 'emitter' }>
  onOpenParticleEditor: (nodeId: string) => void
  onChange: (node: SceneNode) => void
}) {
  const originX = resolveEmitterOriginX(node)
  const originY = resolveEmitterOriginY(node)
  const directionDeg = resolveEmitterDirectionDeg(node)
  const colorHex = resolveEmitterColorHex(node)
  const shape = resolveEmitterShape(node)
  const minSpeed = resolveEmitterMinSpeed(node)
  const maxSpeed = resolveEmitterMaxSpeed(node)
  const minLife = resolveEmitterMinLife(node)
  const maxLife = resolveEmitterMaxLife(node)
  const particleRotationDeg = resolveEmitterParticleRotationDeg(node)
  const sizeCurve = resolveScalarCurve(node.size_curve, defaultEmitterSizeCurve(node.preset))
  const alphaCurve = resolveScalarCurve(node.alpha_curve, defaultEmitterAlphaCurve(node.preset))
  const colorCurve = resolveColorCurve(node.color_curve, defaultEmitterColorCurve(node.preset))
  const particleImageKey = resolveEmitterParticleImageKey(node)
  return (
    <div className="space-y-4">
      <ComposerSection title="Basic" defaultOpen>
        <div className="grid grid-cols-2 gap-2">
          {SCENE_EMITTER_OPTIONS.map((option) => (
            <button
              key={option.preset}
              className={['secondary-button justify-center', node.preset === option.preset ? 'border-cyan-300/40 text-cyan-100' : ''].join(' ')}
              onClick={() => onChange({
                ...node,
                preset: option.preset,
                shape: defaultEmitterShape(option.preset),
                direction_deg: defaultEmitterDirectionDeg(option.preset),
                color_hex: defaultEmitterColorHex(option.preset),
                region_width: defaultEmitterRegionWidth(option.preset),
                region_height: defaultEmitterRegionHeight(option.preset),
                region_radius: defaultEmitterRegionRadius(option.preset),
                line_length: defaultEmitterLineLength(option.preset),
                line_angle_deg: defaultEmitterDirectionDeg(option.preset),
                min_speed: defaultEmitterMinSpeed(option.preset),
                max_speed: defaultEmitterMaxSpeed(option.preset),
                min_life: defaultEmitterMinLife(option.preset),
                max_life: defaultEmitterMaxLife(option.preset),
                particle_rotation_deg: 0,
                size_curve: defaultEmitterSizeCurve(option.preset),
                alpha_curve: defaultEmitterAlphaCurve(option.preset),
                color_curve: defaultEmitterColorCurve(option.preset),
              })}
              type="button"
            >
              {option.label}
            </button>
          ))}
        </div>
        <div>
          <div className="mb-2 text-xs uppercase tracking-[0.18em] text-slate-400">Emitter shape</div>
          <div className="grid grid-cols-2 gap-2">
            {(['point', 'box', 'line', 'circle'] as const).map((option) => (
              <button
                key={option}
                className={['secondary-button justify-center capitalize', shape === option ? 'border-cyan-300/40 text-cyan-100' : ''].join(' ')}
                onClick={() => onChange({ ...node, shape: option })}
                type="button"
              >
                {option}
              </button>
            ))}
          </div>
        </div>
        <div className="grid gap-3 sm:grid-cols-2">
          <RangeField
            label="Emitter X"
            max={100}
            min={0}
            step={1}
            value={originX * 100}
            onChange={(value) => onChange({ ...node, origin_x: value / 100 })}
          />
          <RangeField
            label="Emitter Y"
            max={100}
            min={0}
            step={1}
            value={originY * 100}
            onChange={(value) => onChange({ ...node, origin_y: value / 100 })}
          />
        </div>
      </ComposerSection>
      <ComposerSection title="Region" defaultOpen>
        <div className="space-y-4">
          {shape === 'box' ? (
            <div className="grid gap-3 sm:grid-cols-2">
              <RangeField label="Width" max={100} min={1} step={1} value={resolveEmitterRegionWidth(node) * 100} onChange={(value) => onChange({ ...node, region_width: value / 100 })} />
              <RangeField label="Height" max={100} min={1} step={1} value={resolveEmitterRegionHeight(node) * 100} onChange={(value) => onChange({ ...node, region_height: value / 100 })} />
            </div>
          ) : null}
          {shape === 'line' ? (
            <div className="grid gap-3 sm:grid-cols-2">
              <RangeField label="Line length" max={100} min={1} step={1} value={resolveEmitterLineLength(node) * 100} onChange={(value) => onChange({ ...node, line_length: value / 100 })} />
              <RangeField label="Line angle" max={180} min={-180} step={1} value={resolveEmitterLineAngleDeg(node)} onChange={(value) => onChange({ ...node, line_angle_deg: clampDegrees(value) })} />
            </div>
          ) : null}
          {shape === 'circle' ? (
            <RangeField label="Radius" max={100} min={1} step={1} value={resolveEmitterRegionRadius(node) * 100} onChange={(value) => onChange({ ...node, region_radius: value / 100 })} />
          ) : null}
          {shape === 'point' ? (
            <div className="surface-muted p-3 text-sm text-slate-300">Point emitters spawn from a single anchor in the viewport.</div>
          ) : null}
        </div>
      </ComposerSection>
      <ComposerSection title="Appearance" defaultOpen>
        <div className="space-y-4">
          <label className="block">
            <div className="mb-2 flex items-center justify-between text-xs uppercase tracking-[0.16em] text-slate-400">
              <span>Tint</span>
              <span>{colorHex}</span>
            </div>
            <input
              className="h-10 w-full border border-white/10 bg-transparent p-1"
              type="color"
              value={colorHex}
              onChange={(event) => onChange({ ...node, color_hex: event.currentTarget.value })}
            />
          </label>
          <RangeField
            label="Particle rotation"
            max={180}
            min={-180}
            step={1}
            value={particleRotationDeg}
            onChange={(value) => onChange({ ...node, particle_rotation_deg: clampDegrees(value) })}
          />
          <div>
            <div className="mb-2 text-xs uppercase tracking-[0.18em] text-slate-400">Particle image</div>
            <div className="grid gap-2">
              <button
                className={['secondary-button justify-center', particleImageKey === null ? 'border-cyan-300/40 text-cyan-100' : ''].join(' ')}
                onClick={() => onChange({ ...node, particle_image_key: null })}
                type="button"
              >
                Procedural particle
              </button>
              {availableImageKeys.filter((image) => image.key !== 'base').map((image) => (
                <button
                  key={image.key}
                  className={['secondary-button justify-center', particleImageKey === image.key ? 'border-cyan-300/40 text-cyan-100' : ''].join(' ')}
                  onClick={() => onChange({ ...node, particle_image_key: image.key })}
                  type="button"
                >
                  {image.label}
                </button>
              ))}
            </div>
          </div>
          <RangeField label="Opacity" max={1} min={0} step={0.01} value={node.opacity} onChange={(value) => onChange({ ...node, opacity: value })} />
          <RangeField label="Particle size" max={12} min={1} step={0.1} value={node.size} onChange={(value) => onChange({ ...node, size: value })} />
        </div>
      </ComposerSection>
      <ComposerSection title="Emission" defaultOpen>
        <div className="space-y-4">
          <RangeField label="Emission rate" max={160} min={1} step={1} value={node.emission_rate} onChange={(value) => onChange({ ...node, emission_rate: value })} />
          <RangeField label="Burst count" max={128} min={0} step={1} value={node.burst_count} onChange={(value) => onChange({ ...node, burst_count: value })} />
          <label className="flex items-center justify-between gap-3 text-sm text-slate-300">
            <span>Burst on start</span>
            <input
              checked={node.burst_on_start}
              className="size-4 accent-emerald-300"
              onChange={(event) => onChange({ ...node, burst_on_start: event.currentTarget.checked })}
              type="checkbox"
            />
          </label>
          <RangeField label="Max particles" max={600} min={8} step={1} value={node.max_particles} onChange={(value) => onChange({ ...node, max_particles: value })} />
          <div className="grid gap-3 sm:grid-cols-2">
            <RangeField label="Min lifetime" max={12} min={0.2} step={0.1} value={minLife} onChange={(value) => onChange({ ...node, min_life: value })} />
            <RangeField label="Max lifetime" max={16} min={0.2} step={0.1} value={maxLife} onChange={(value) => onChange({ ...node, max_life: Math.max(value, minLife) })} />
          </div>
        </div>
      </ComposerSection>
      <ComposerSection title="Motion" defaultOpen>
        <div className="space-y-4">
          <RangeField
            label="Direction"
            max={180}
            min={-180}
            step={1}
            value={directionDeg}
            onChange={(value) => onChange({ ...node, direction_deg: clampDegrees(value) })}
          />
          <div className="grid gap-3 sm:grid-cols-2">
            <RangeField label="Min speed" max={900} min={0} step={1} value={minSpeed} onChange={(value) => onChange({ ...node, min_speed: value })} />
            <RangeField label="Max speed" max={1200} min={0} step={1} value={maxSpeed} onChange={(value) => onChange({ ...node, max_speed: Math.max(value, minSpeed) })} />
          </div>
          <RangeField label="Spread" max={180} min={0} step={1} value={node.spread} onChange={(value) => onChange({ ...node, spread: value })} />
          <div className="grid gap-3 sm:grid-cols-2">
            <RangeField label="Gravity X" max={320} min={-320} step={1} value={node.gravity_x} onChange={(value) => onChange({ ...node, gravity_x: value })} />
            <RangeField label="Gravity Y" max={320} min={-320} step={1} value={node.gravity_y} onChange={(value) => onChange({ ...node, gravity_y: value })} />
          </div>
          <RangeField label="Drag" max={8} min={0} step={0.05} value={node.drag} onChange={(value) => onChange({ ...node, drag: value })} />
        </div>
      </ComposerSection>
      <ComposerSection title="Curves">
        <div className="space-y-4">
          <div className="surface-muted p-3 text-sm text-slate-300">
            Use the dedicated particle editor for curve shaping and live particle preview. The inline inspector keeps only the core emitter controls.
          </div>
          <button
            className="secondary-button w-full justify-center"
            onClick={() => onOpenParticleEditor(node.id)}
            type="button"
          >
            <FiSliders className="size-4" />
            Open particle editor
          </button>
          <div className="grid gap-3 sm:grid-cols-3">
            <CurveSummaryCard label="Size" points={sizeCurve.length} />
            <CurveSummaryCard label="Alpha" points={alphaCurve.length} />
            <CurveSummaryCard label="Color" points={colorCurve.length} />
          </div>
        </div>
      </ComposerSection>
    </div>
  )
}

function ComposerSection({
  children,
  defaultOpen = false,
  title,
}: {
  children: ReactNode
  defaultOpen?: boolean
  title: string
}) {
  const [open, setOpen] = useState(defaultOpen)
  return (
    <div className="surface-muted overflow-hidden">
      <button
        className="flex w-full items-center justify-between px-4 py-3 text-left"
        onClick={() => setOpen((value) => !value)}
        type="button"
      >
        <span className="text-xs uppercase tracking-[0.18em] text-slate-400">{title}</span>
        <span className="text-sm text-slate-500">{open ? '−' : '+'}</span>
      </button>
      {open ? <div className="space-y-4 border-t border-white/10 px-4 py-4">{children}</div> : null}
    </div>
  )
}

function RangeField({
  label,
  max,
  min,
  onChange,
  step,
  value,
}: {
  label: string
  max: number
  min: number
  onChange: (value: number) => void
  step: number
  value: number
}) {
  return (
    <label className="block">
      <div className="mb-2 flex items-center justify-between text-xs uppercase tracking-[0.16em] text-slate-400">
        <span>{label}</span>
        <span>{value.toFixed(step >= 1 ? 0 : 2)}</span>
      </div>
      <input
        className="w-full accent-emerald-300"
        max={max}
        min={min}
        onChange={(event) => onChange(Number(event.currentTarget.value))}
        step={step}
        type="range"
        value={value}
      />
    </label>
  )
}

function CurveSummaryCard({ label, points }: { label: string; points: number }) {
  return (
    <div className="surface-muted p-3">
      <div className="text-xs uppercase tracking-[0.16em] text-slate-400">{label}</div>
      <div className="mt-1 text-sm font-medium text-slate-100">{points} control points</div>
    </div>
  )
}

function ParticleEditorModal({
  availableImageKeys,
  documentVisible,
  node,
  onClose,
  onSave,
}: {
  availableImageKeys: Array<{ key: string; label: string }>
  documentVisible: boolean
  node: Extract<SceneNode, { kind: 'emitter' }>
  onClose: () => void
  onSave: (node: Extract<SceneNode, { kind: 'emitter' }>) => void
}) {
  const [draft, setDraft] = useState(node)
  const [sizePoints, setSizePoints] = useState(() => toEditableScalarPoints(resolveScalarCurve(node.size_curve, defaultEmitterSizeCurve(node.preset))))
  const [alphaPoints, setAlphaPoints] = useState(() => toEditableScalarPoints(resolveScalarCurve(node.alpha_curve, defaultEmitterAlphaCurve(node.preset))))
  const [colorStops, setColorStops] = useState(() => toEditableColorStops(resolveColorCurve(node.color_curve, defaultEmitterColorCurve(node.preset))))

  useEffect(() => {
    setDraft(node)
    setSizePoints(toEditableScalarPoints(resolveScalarCurve(node.size_curve, defaultEmitterSizeCurve(node.preset))))
    setAlphaPoints(toEditableScalarPoints(resolveScalarCurve(node.alpha_curve, defaultEmitterAlphaCurve(node.preset))))
    setColorStops(toEditableColorStops(resolveColorCurve(node.color_curve, defaultEmitterColorCurve(node.preset))))
  }, [node])

  const previewNode: SceneEmitterNode = {
    ...draft,
    size_curve: fromEditableScalarPoints(sizePoints),
    alpha_curve: fromEditableScalarPoints(alphaPoints),
    color_curve: fromEditableColorStops(colorStops),
  }

  return (
    <div className="absolute inset-0 z-20 flex items-center justify-center bg-slate-950/82 p-6 backdrop-blur-sm">
      <div className="surface-panel flex h-full w-full max-w-[1200px] flex-col overflow-hidden">
        <div className="flex items-center justify-between border-b border-white/10 px-5 py-4">
          <div>
            <div className="section-title">Particle Editor</div>
            <div className="mt-1 text-sm text-slate-400">Tune particle curves and preview emitter behavior without fighting the main composer inspector.</div>
          </div>
          <div className="flex items-center gap-3">
            <button className="secondary-button" onClick={onClose} type="button">Cancel</button>
            <button className="primary-button" onClick={() => onSave(previewNode)} type="button">
              <FiDisc className="size-4" />
              Apply particle changes
            </button>
          </div>
        </div>
        <div className="grid min-h-0 flex-1 gap-0 xl:grid-cols-[1.1fr_0.9fr]">
          <div className="flex min-h-0 flex-col border-r border-white/10">
            <div className="border-b border-white/10 px-5 py-4">
              <div className="text-sm font-medium text-white">{draft.name}</div>
              <div className="mt-1 text-xs uppercase tracking-[0.16em] text-slate-400">{draft.preset} emitter</div>
            </div>
            <div className="min-h-0 flex-1 p-5">
              <ParticlePreviewPanel
                documentVisible={documentVisible}
                node={previewNode}
              />
            </div>
          </div>
          <div className="min-h-0 overflow-y-auto p-5">
            <div className="space-y-4">
              <ComposerSection title="Emitter" defaultOpen>
                <div className="grid gap-3 sm:grid-cols-2">
                  <RangeField label="Min speed" max={900} min={0} step={1} value={resolveEmitterMinSpeed(draft)} onChange={(value) => setDraft((current) => ({ ...current, min_speed: value }))} />
                  <RangeField label="Max speed" max={1200} min={0} step={1} value={resolveEmitterMaxSpeed(draft)} onChange={(value) => setDraft((current) => ({ ...current, max_speed: Math.max(value, resolveEmitterMinSpeed(current)) }))} />
                  <RangeField label="Min lifetime" max={12} min={0.2} step={0.1} value={resolveEmitterMinLife(draft)} onChange={(value) => setDraft((current) => ({ ...current, min_life: value }))} />
                  <RangeField label="Max lifetime" max={16} min={0.2} step={0.1} value={resolveEmitterMaxLife(draft)} onChange={(value) => setDraft((current) => ({ ...current, max_life: Math.max(value, resolveEmitterMinLife(current)) }))} />
                </div>
                <div>
                  <div className="mb-2 text-xs uppercase tracking-[0.18em] text-slate-400">Particle image</div>
                  <div className="grid gap-2">
                    <button
                      className={['secondary-button justify-center', resolveEmitterParticleImageKey(draft) === null ? 'border-cyan-300/40 text-cyan-100' : ''].join(' ')}
                      onClick={() => setDraft((current) => ({ ...current, particle_image_key: null }))}
                      type="button"
                    >
                      Procedural particle
                    </button>
                    {availableImageKeys.filter((image) => image.key !== 'base').map((image) => (
                      <button
                        key={image.key}
                        className={['secondary-button justify-center', resolveEmitterParticleImageKey(draft) === image.key ? 'border-cyan-300/40 text-cyan-100' : ''].join(' ')}
                        onClick={() => setDraft((current) => ({ ...current, particle_image_key: image.key }))}
                        type="button"
                      >
                        {image.label}
                      </button>
                    ))}
                  </div>
                </div>
              </ComposerSection>
              <ScalarCurveModalEditor curve={sizePoints} label="Size over lifetime" maxY={2} onChange={setSizePoints} />
              <ScalarCurveModalEditor curve={alphaPoints} label="Alpha over lifetime" maxY={1} onChange={setAlphaPoints} />
              <ColorCurveModalEditor curve={colorStops} label="Color over lifetime" onChange={setColorStops} />
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

function ParticlePreviewPanel({
  documentVisible,
  node,
}: {
  documentVisible: boolean
  node: Extract<SceneNode, { kind: 'emitter' }>
}) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null)
  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) {
      return
    }
    const context = canvas.getContext('2d')
    if (!context) {
      return
    }
    let frame = 0
    let disposed = false
    const frameIntervalMs = 1000 / 24
    let lastRenderedAt = 0
    const render = (now: number) => {
      if (disposed) {
        return
      }
      if (!documentVisible) {
        frame = window.requestAnimationFrame(render)
        return
      }
      if (now - lastRenderedAt < frameIntervalMs) {
        frame = window.requestAnimationFrame(render)
        return
      }
      lastRenderedAt = now
      if (canvas.width !== 960 || canvas.height !== 540) {
        canvas.width = 960
        canvas.height = 540
      }
      context.clearRect(0, 0, canvas.width, canvas.height)
      const gradient = context.createLinearGradient(0, 0, canvas.width, canvas.height)
      gradient.addColorStop(0, '#040810')
      gradient.addColorStop(1, '#0b1522')
      context.fillStyle = gradient
      context.fillRect(0, 0, canvas.width, canvas.height)
      drawPreviewGrid(context, canvas.width, canvas.height)
      drawComposerEmitterNode(context, new Map<string, HTMLImageElement>(), node, [], now / 1000)
      frame = window.requestAnimationFrame(render)
    }
    frame = window.requestAnimationFrame(render)
    return () => {
      disposed = true
      window.cancelAnimationFrame(frame)
    }
  }, [documentVisible, node])

  return (
    <div className="flex h-full flex-col gap-3">
      <div className="text-xs uppercase tracking-[0.16em] text-slate-400">Particle preview</div>
      <div className="relative flex-1 overflow-hidden border border-white/10 bg-slate-950/90">
        <canvas ref={canvasRef} className="h-full w-full object-contain" width={960} height={540} />
      </div>
    </div>
  )
}

function ScalarCurveModalEditor({
  curve,
  label,
  maxY,
  onChange,
}: {
  curve: EditableScalarPoint[]
  label: string
  maxY: number
  onChange: (curve: EditableScalarPoint[]) => void
}) {
  const path = curve.map((point, index) => {
    const x = point.x * 100
    const y = 100 - ((point.y / Math.max(maxY, 0.01)) * 100)
    return `${index === 0 ? 'M' : 'L'} ${x},${y}`
  }).join(' ')
  return (
    <ComposerSection title={label} defaultOpen>
      <div className="space-y-4">
        <svg className="h-24 w-full border border-white/10 bg-slate-950/70" viewBox="0 0 100 100" preserveAspectRatio="none">
          <path d={path} fill="none" stroke="rgb(110 231 183)" strokeWidth="2" />
          {curve.map((point) => (
            <circle key={point.id} cx={point.x * 100} cy={100 - ((point.y / Math.max(maxY, 0.01)) * 100)} r="2.4" fill="rgb(103 232 249)" />
          ))}
        </svg>
        {curve.map((point, index) => (
          <div key={point.id} className="grid gap-3 sm:grid-cols-2">
            <RangeField
              label={`Point ${index + 1} time`}
              max={index === 0 || index === curve.length - 1 ? point.x : Math.min(1, curve[index + 1].x - 0.01)}
              min={index === 0 || index === curve.length - 1 ? point.x : Math.max(0, curve[index - 1].x + 0.01)}
              step={0.01}
              value={point.x}
              onChange={(value) => onChange(curve.map((entry) => entry.id === point.id ? { ...entry, x: value } : entry))}
            />
            <RangeField
              label={`Point ${index + 1} value`}
              max={maxY}
              min={0}
              step={0.01}
              value={point.y}
              onChange={(value) => onChange(curve.map((entry) => entry.id === point.id ? { ...entry, y: value } : entry))}
            />
          </div>
        ))}
      </div>
    </ComposerSection>
  )
}

function ColorCurveModalEditor({
  curve,
  label,
  onChange,
}: {
  curve: EditableColorStop[]
  label: string
  onChange: (curve: EditableColorStop[]) => void
}) {
  const gradient = curve.map((stop) => `${stop.color_hex} ${Math.round(stop.x * 100)}%`).join(', ')
  return (
    <ComposerSection title={label} defaultOpen>
      <div className="space-y-4">
        <div className="h-12 border border-white/10" style={{ background: `linear-gradient(90deg, ${gradient})` }} />
        {curve.map((stop, index) => (
          <div key={stop.id} className="grid gap-3 sm:grid-cols-[1fr_auto]">
            <RangeField
              label={`Stop ${index + 1} time`}
              max={index === 0 || index === curve.length - 1 ? stop.x : Math.min(1, curve[index + 1].x - 0.01)}
              min={index === 0 || index === curve.length - 1 ? stop.x : Math.max(0, curve[index - 1].x + 0.01)}
              step={0.01}
              value={stop.x}
              onChange={(value) => onChange(curve.map((entry) => entry.id === stop.id ? { ...entry, x: value } : entry))}
            />
            <label className="block">
              <div className="mb-2 text-xs uppercase tracking-[0.16em] text-slate-400">Color</div>
              <input
                className="h-10 w-16 border border-white/10 bg-transparent p-1"
                type="color"
                value={stop.color_hex}
                onChange={(event) => onChange(curve.map((entry) => entry.id === stop.id ? { ...entry, color_hex: event.currentTarget.value } : entry))}
              />
            </label>
          </div>
        ))}
      </div>
    </ComposerSection>
  )
}

function toEditableScalarPoints(points: SceneCurvePoint[]): EditableScalarPoint[] {
  return points.map((point, index) => ({
    id: `curve-${index}-${point.x.toFixed(3)}-${point.y.toFixed(3)}`,
    x: point.x,
    y: point.y,
  }))
}

function toEditableColorStops(points: SceneColorStop[]): EditableColorStop[] {
  return points.map((point, index) => ({
    id: `stop-${index}-${point.x.toFixed(3)}-${point.color_hex}`,
    x: point.x,
    color_hex: normalizeColorHex(point.color_hex) ?? '#ffffff',
  }))
}

function fromEditableScalarPoints(points: EditableScalarPoint[]): SceneCurvePoint[] {
  return points
    .map((point) => ({ x: clamp01(point.x), y: Math.max(0, point.y) }))
    .sort((left, right) => left.x - right.x)
}

function fromEditableColorStops(points: EditableColorStop[]): SceneColorStop[] {
  return points
    .map((point) => ({ x: clamp01(point.x), color_hex: normalizeColorHex(point.color_hex) ?? '#ffffff' }))
    .sort((left, right) => left.x - right.x)
}

function drawPreviewGrid(context: CanvasRenderingContext2D, width: number, height: number) {
  context.save()
  context.strokeStyle = 'rgba(148, 163, 184, 0.14)'
  context.lineWidth = 1
  for (let x = 0; x < width; x += 48) {
    context.beginPath()
    context.moveTo(x, 0)
    context.lineTo(x, height)
    context.stroke()
  }
  for (let y = 0; y < height; y += 48) {
    context.beginPath()
    context.moveTo(0, y)
    context.lineTo(width, y)
    context.stroke()
  }
  context.restore()
}

function toggleSpriteBehavior(node: SceneSpriteNode, kind: SceneBehaviorKind): SceneNode {
  const existing = node.behaviors.find((behavior) => behavior.kind === kind)
  if (existing) {
    return { ...node, behaviors: node.behaviors.filter((behavior) => behavior.kind !== kind) }
  }
  return {
    ...node,
    behaviors: [
      ...node.behaviors,
      {
        kind,
        speed: 1,
        amount_x: kind === 'drift' ? 24 : 0,
        amount_y: kind === 'drift' ? 12 : 0,
        amount: kind === 'pulse' ? 0.08 : 38,
        phase: 0,
      },
    ],
  }
}

function updateSpriteBehavior(
  node: SceneSpriteNode,
  kind: SceneBehaviorKind,
  patch: Partial<SceneSpriteNode['behaviors'][number]>,
): SceneNode {
  return {
    ...node,
    behaviors: node.behaviors.map((behavior) => (
      behavior.kind === kind ? { ...behavior, ...patch } : behavior
    )),
  }
}

function drawComposerSceneFrame(
  context: CanvasRenderingContext2D,
  images: Map<string, HTMLImageElement>,
  nodes: SceneNode[],
  selectedNode: SceneNode | null,
  draftParticleRegion: SceneNormalizedRect | null,
  timeSeconds: number,
) {
  const width = context.canvas.width
  const height = context.canvas.height
  context.clearRect(0, 0, width, height)
  context.fillStyle = '#05070a'
  context.fillRect(0, 0, width, height)
  const particleBlockers = buildComposerParticleBlockers(images, nodes, width, height, timeSeconds)

  for (const node of nodes) {
    if (!node.enabled) {
      continue
    }
    if (node.kind === 'sprite') {
      const image = images.get(node.image_key)
      if (image) {
        drawComposerSpriteNode(context, image, node, timeSeconds)
      }
    } else if (node.kind === 'effect') {
      drawComposerEffectNode(context, node, timeSeconds)
    } else if (node.kind === 'emitter') {
      drawComposerEmitterNode(context, images, node, particleBlockers, timeSeconds)
    }
  }

  if (selectedNode?.kind === 'sprite') {
    const image = images.get(selectedNode.image_key)
    if (image) {
      const layout = resolveComposerSpriteLayout(context, image, selectedNode, timeSeconds)
      const region = resolveSpriteParticleRegionRect(
        layout,
        draftParticleRegion ?? selectedNode.particle_region ?? null,
      )
      if (region) {
        drawParticleRegionOverlay(context, region)
      }
    }
  }
  if (selectedNode?.kind === 'particle_area') {
    if ((selectedNode.shape ?? 'rect') === 'polygon') {
      drawParticlePolygonOverlay(
        context,
        (selectedNode.points ?? []).map((point) => ({
          x: context.canvas.width * point.x,
          y: context.canvas.height * point.y,
        })),
      )
    } else {
      drawParticleRegionOverlay(context, {
        x: context.canvas.width * (draftParticleRegion?.x ?? selectedNode.region.x),
        y: context.canvas.height * (draftParticleRegion?.y ?? selectedNode.region.y),
        width: context.canvas.width * (draftParticleRegion?.width ?? selectedNode.region.width),
        height: context.canvas.height * (draftParticleRegion?.height ?? selectedNode.region.height),
      })
    }
  }
}

function resolveComposerSpriteLayoutFromBounds(
  canvasWidth: number,
  canvasHeight: number,
  image: HTMLImageElement,
  node: SceneSpriteNode,
  timeSeconds: number,
) {
  const canvasAspect = canvasWidth / Math.max(canvasHeight, 1)
  const sourceAspect = image.naturalWidth / Math.max(image.naturalHeight, 1)
  let offsetX = node.x
  let offsetY = node.y
  let opacity = node.opacity
  let scale = node.scale

  for (const behavior of node.behaviors) {
    const phase = timeSeconds * behavior.speed + behavior.phase
    if (behavior.kind === 'drift') {
      offsetX += Math.sin(phase) * behavior.amount_x
      offsetY += Math.cos(phase * 0.9) * behavior.amount_y
    }
    if (behavior.kind === 'pulse') {
      scale += Math.sin(phase) * behavior.amount
      opacity *= 0.88 + ((Math.sin(phase) + 1) * 0.06)
    }
    if (behavior.kind === 'orbit') {
      offsetX += Math.cos(phase) * behavior.amount
      offsetY += Math.sin(phase) * Math.max(behavior.amount * 0.6, 0)
    }
  }

  const fit = node.fit ?? 'cover'
  let drawWidth = image.naturalWidth
  let drawHeight = image.naturalHeight

  if (fit === 'cover') {
    if (sourceAspect > canvasAspect) {
      drawHeight = canvasHeight
      drawWidth = drawHeight * sourceAspect
    } else {
      drawWidth = canvasWidth
      drawHeight = drawWidth / sourceAspect
    }
  } else if (fit === 'contain') {
    if (sourceAspect > canvasAspect) {
      drawWidth = canvasWidth
      drawHeight = drawWidth / sourceAspect
    } else {
      drawHeight = canvasHeight
      drawWidth = drawHeight * sourceAspect
    }
  } else if (fit === 'stretch') {
    drawWidth = canvasWidth
    drawHeight = canvasHeight
  }

  if (fit !== 'center') {
    drawWidth *= scale
    drawHeight *= scale
  }

  return {
    x: ((canvasWidth - drawWidth) / 2) + offsetX,
    y: ((canvasHeight - drawHeight) / 2) + offsetY,
    width: drawWidth,
    height: drawHeight,
    opacity,
  }
}

function resolveComposerSpriteLayout(
  context: CanvasRenderingContext2D,
  image: HTMLImageElement,
  node: SceneSpriteNode,
  timeSeconds: number,
) {
  return resolveComposerSpriteLayoutFromBounds(context.canvas.width, context.canvas.height, image, node, timeSeconds)
}

function normalizeDraggedRect(startX: number, startY: number, endX: number, endY: number): SceneNormalizedRect {
  const x = Math.min(startX, endX)
  const y = Math.min(startY, endY)
  const width = Math.max(0.001, Math.abs(endX - startX))
  const height = Math.max(0.001, Math.abs(endY - startY))
  return {
    x: clamp01(x),
    y: clamp01(y),
    width: Math.min(width, 1 - clamp01(x)),
    height: Math.min(height, 1 - clamp01(y)),
  }
}

function polygonBounds(points: SceneNormalizedPoint[]): SceneNormalizedRect | null {
  if (points.length === 0) {
    return null
  }
  const xs = points.map((point) => point.x)
  const ys = points.map((point) => point.y)
  const minX = Math.min(...xs)
  const maxX = Math.max(...xs)
  const minY = Math.min(...ys)
  const maxY = Math.max(...ys)
  return {
    x: minX,
    y: minY,
    width: Math.max(0.001, maxX - minX),
    height: Math.max(0.001, maxY - minY),
  }
}

function resolveSpriteParticleRegionRect(
  layout: { x: number; y: number; width: number; height: number },
  region: SceneNormalizedRect | null,
) {
  if (!region) {
    return layout
  }
  return {
    x: layout.x + (layout.width * region.x),
    y: layout.y + (layout.height * region.y),
    width: layout.width * region.width,
    height: layout.height * region.height,
  }
}

function drawParticleRegionOverlay(
  context: CanvasRenderingContext2D,
  region: { x: number; y: number; width: number; height: number },
) {
  context.save()
  context.strokeStyle = 'rgba(103, 232, 249, 0.95)'
  context.fillStyle = 'rgba(103, 232, 249, 0.12)'
  context.lineWidth = 2
  context.setLineDash([8, 6])
  context.fillRect(region.x, region.y, region.width, region.height)
  context.strokeRect(region.x, region.y, region.width, region.height)
  context.restore()
}

function drawParticlePolygonOverlay(
  context: CanvasRenderingContext2D,
  points: Array<{ x: number; y: number }>,
) {
  if (points.length === 0) {
    return
  }
  context.save()
  context.strokeStyle = 'rgba(103, 232, 249, 0.95)'
  context.fillStyle = 'rgba(103, 232, 249, 0.12)'
  context.lineWidth = 2
  context.setLineDash([8, 6])
  context.beginPath()
  context.moveTo(points[0].x, points[0].y)
  for (const point of points.slice(1)) {
    context.lineTo(point.x, point.y)
  }
  context.closePath()
  context.fill()
  context.stroke()
  context.setLineDash([])
  for (const point of points) {
    context.beginPath()
    context.arc(point.x, point.y, 4, 0, Math.PI * 2)
    context.fillStyle = 'rgba(103, 232, 249, 0.95)'
    context.fill()
  }
  context.restore()
}

function drawComposerSpriteNode(
  context: CanvasRenderingContext2D,
  image: HTMLImageElement,
  node: SceneSpriteNode,
  timeSeconds: number,
) {
  const layout = resolveComposerSpriteLayout(context, image, node, timeSeconds)

  context.save()
  context.globalAlpha = layout.opacity
  context.translate(layout.x + layout.width / 2, layout.y + layout.height / 2)
  context.rotate((node.rotation_deg * Math.PI) / 180)
  context.drawImage(image, -layout.width / 2, -layout.height / 2, layout.width, layout.height)
  context.restore()
}

function drawComposerEffectNode(
  context: CanvasRenderingContext2D,
  node: Extract<SceneNode, { kind: 'effect' }>,
  timeSeconds: number,
) {
  const width = context.canvas.width
  const height = context.canvas.height
  const { red, green, blue } = parseColorHex(resolveEffectColorHex(node))
  context.save()
  if (node.effect === 'glow') {
    const pulse = node.intensity * (0.78 + ((Math.sin(timeSeconds * Math.max(node.speed, 0.01)) + 1) * 0.11))
    const gradient = context.createRadialGradient(
      width * 0.5,
      height * 0.5,
      0,
      width * 0.5,
      height * 0.5,
      Math.max(width, height) * 0.61,
    )
    gradient.addColorStop(0, `rgba(${red}, ${green}, ${blue}, ${node.opacity * pulse})`)
    gradient.addColorStop(0.45, `rgba(${red}, ${green}, ${blue}, ${0.34 * node.opacity * pulse})`)
    gradient.addColorStop(1, 'rgba(255, 199, 133, 0)')
    context.fillStyle = gradient
    context.fillRect(0, 0, width, height)
  } else if (node.effect === 'vignette') {
    const gradient = context.createRadialGradient(
      width * 0.5,
      height * 0.5,
      Math.min(width, height) * 0.22,
      width * 0.5,
      height * 0.5,
      Math.hypot(width * 0.5, height * 0.5),
    )
    gradient.addColorStop(0, `rgba(${red}, ${green}, ${blue}, 0)`)
    gradient.addColorStop(0.42, `rgba(${red}, ${green}, ${blue}, 0)`)
    gradient.addColorStop(1, `rgba(${red}, ${green}, ${blue}, ${node.opacity * node.intensity})`)
    context.fillStyle = gradient
    context.fillRect(0, 0, width, height)
  } else if (node.effect === 'scanlines') {
    const offset = (timeSeconds * Math.max(node.speed, 0.01) * 0.35) % 1
    const alphaScale = node.opacity * node.intensity * 0.18
    const lineStep = height / 96
    context.fillStyle = `rgb(${red}, ${green}, ${blue})`
    for (let y = 0; y < height; y += 1) {
      const linePhase = ((((y / Math.max(height, 1)) + offset) * 96) % 1 + 1) % 1
      const distanceToCenter = Math.abs(linePhase - 0.5)
      const band = 1 - smoothstep(0.28, 0.5, distanceToCenter)
      const alpha = band * alphaScale
      if (alpha <= 0.001) {
        continue
      }
      context.globalAlpha = alpha
      context.fillRect(0, y, width, Math.max(1, lineStep * 0.2))
    }
    context.globalAlpha = 1
  } else if (node.effect === 'fog') {
    const cell = Math.max(2, Math.floor(Math.min(width, height) / 180))
    context.fillStyle = `rgb(${red}, ${green}, ${blue})`
    for (let y = 0; y < height; y += cell) {
      for (let x = 0; x < width; x += cell) {
        const uvx = x / Math.max(width, 1)
        const uvy = y / Math.max(height, 1)
        const fogWave = Math.sin((uvx * 5) + (timeSeconds * Math.max(node.speed, 0.01))) * 0.03
        const band = smoothstep(0.12 + fogWave, 0.72 + fogWave, uvy) * (1 - smoothstep(0.56 + fogWave, 1, uvy))
        const alpha = band * node.opacity * node.intensity * 0.22
        if (alpha <= 0.001) {
          continue
        }
        context.globalAlpha = alpha
        context.fillRect(x, y, cell, cell)
      }
    }
    context.globalAlpha = 1
  }
  context.restore()
}

function drawComposerEmitterNode(
  context: CanvasRenderingContext2D,
  images: Map<string, HTMLImageElement>,
  node: Extract<SceneNode, { kind: 'emitter' }>,
  particleBlockers: ComposerParticleBlocker[],
  timeSeconds: number,
) {
  const particles = buildComposerEmitterParticles(node, context.canvas.width, context.canvas.height, particleBlockers, timeSeconds)
  const particleImageKey = resolveEmitterParticleImageKey(node)
  const particleImage = particleImageKey ? images.get(particleImageKey) ?? null : null
  context.save()
  for (const particle of particles) {
    context.globalAlpha = particle.alpha
    if (particle.shape === 'texture' && particleImage) {
      context.save()
      context.translate(particle.x, particle.y)
      context.rotate(particle.angle)
      context.drawImage(particleImage, -particle.sizeX * 0.5, -particle.sizeY * 0.5, particle.sizeX, particle.sizeY)
      context.restore()
    } else if (particle.shape === 'streak') {
      context.save()
      context.translate(particle.x, particle.y)
      context.rotate(particle.angle)
      context.fillStyle = particle.color
      context.fillRect(-particle.sizeX * 0.5, -particle.sizeY * 0.5, particle.sizeX, particle.sizeY)
      context.restore()
    } else {
      context.fillStyle = particle.color
      context.beginPath()
      context.arc(particle.x, particle.y, particle.radius, 0, Math.PI * 2)
      context.fill()
    }
  }
  context.restore()
}

function buildComposerEmitterParticles(
  node: Extract<SceneNode, { kind: 'emitter' }>,
  width: number,
  height: number,
  particleBlockers: ComposerParticleBlocker[],
  timeSeconds: number,
) {
  const averageLife = (resolveEmitterMinLife(node) + resolveEmitterMaxLife(node)) / 2
  const count = Math.max(12, Math.min(node.max_particles, Math.round(node.emission_rate * averageLife)))
  const sizeCurve = resolveScalarCurve(node.size_curve, defaultEmitterSizeCurve(node.preset))
  const alphaCurve = resolveScalarCurve(node.alpha_curve, defaultEmitterAlphaCurve(node.preset))
  const colorCurve = resolveColorCurve(node.color_curve, defaultEmitterColorCurve(node.preset))
  const particles: Array<{
    x: number
    y: number
    radius: number
    sizeX: number
    sizeY: number
    angle: number
    alpha: number
    color: string
    shape: 'circle' | 'streak' | 'texture'
  }> = []
  const originX = resolveEmitterOriginX(node) * width
  const originY = resolveEmitterOriginY(node) * height
  const directionRadians = (resolveEmitterDirectionDeg(node) * Math.PI) / 180
  const textured = resolveEmitterParticleImageKey(node) !== null

  for (let index = 0; index < count; index += 1) {
    let seed = stablePreviewSeed(index, node)
    const random = () => nextPreviewRandom(seed = nextPreviewSeed(seed))
    const spawnPosition = sampleEmitterPosition(node, width, height, originX, originY, random)
    const spreadRadians = (node.spread * Math.PI) / 180
    const angle = directionRadians + ((random() - 0.5) * spreadRadians)
    const speed = resolveEmitterMinSpeed(node) + (random() * (resolveEmitterMaxSpeed(node) - resolveEmitterMinSpeed(node)))
    const maxLife = resolveEmitterMinLife(node) + (random() * (resolveEmitterMaxLife(node) - resolveEmitterMinLife(node)))
    const size = node.size * (0.55 + random() * 0.7)
    const alphaBase = node.opacity * (0.55 + random() * 0.45)
    const phase = random() * maxLife
    const age = (timeSeconds + phase) % maxLife
    const dragScale = Math.max(0, 1 - (node.drag * age * 0.08))
    const vx = speed * Math.cos(angle) * dragScale
    const vy = speed * Math.sin(angle) * dragScale
    const x = spawnPosition.x + vx * age + (0.5 * node.gravity_x * age * age)
    let y = spawnPosition.y + vy * age + (0.5 * node.gravity_y * age * age)
    const progress = Math.min(1, age / maxLife)
    const sizeScale = evaluateScalarCurve(sizeCurve, progress)
    const alpha = Math.max(0, alphaBase * evaluateScalarCurve(alphaCurve, progress))
    if (alpha <= 0) {
      continue
    }
    const radius = Math.max(1, size * sizeScale)
    const collision = resolveComposerParticleCollision(node, particleBlockers, x, y, radius)
    if (collision === 'discard') {
      continue
    }
    if (typeof collision === 'number') {
      y = collision
    }
    const color = evaluateColorCurve(colorCurve, progress, resolveEmitterColorHex(node))
    const particleShape = node.preset === 'rain' && !textured ? 'streak' : textured ? 'texture' : 'circle'
    const renderAngle = resolveRenderedParticleAngleRad(node, vx, vy)
    const sizeX = node.preset === 'rain'
      ? size * sizeScale * 1.2
      : node.preset === 'snow'
        ? size * sizeScale * 2.0
        : node.preset === 'dust'
          ? size * sizeScale * 2.2
          : size * sizeScale * 2.0
    const sizeY = node.preset === 'rain'
      ? size * sizeScale * 8.5
      : node.preset === 'snow'
        ? size * sizeScale * 2.0
        : node.preset === 'dust'
          ? size * sizeScale * 2.2
          : size * sizeScale * 2.0
    const occluded = node.preset === 'rain'
      ? isComposerParticleSegmentOccluded(
        particleBlockers,
        { x: x - (Math.cos(renderAngle + (Math.PI / 2)) * sizeY * 0.5), y: y - (Math.sin(renderAngle + (Math.PI / 2)) * sizeY * 0.5) },
        { x: x + (Math.cos(renderAngle + (Math.PI / 2)) * sizeY * 0.5), y: y + (Math.sin(renderAngle + (Math.PI / 2)) * sizeY * 0.5) },
        Math.max(sizeX, 1.5) * 0.5,
      )
      : isComposerParticleOccluded(particleBlockers, x, y, Math.max(sizeX, sizeY) * 0.5)

    if (occluded) {
      continue
    }

    if (node.preset === 'rain') {
      particles.push({
        x,
        y,
        radius: 0,
        sizeX,
        sizeY,
        angle: renderAngle,
        alpha: alpha * 0.92,
        color,
        shape: particleShape,
      })
    } else if (node.preset === 'snow') {
      particles.push({
        x,
        y,
        radius,
        sizeX,
        sizeY,
        angle: renderAngle,
        alpha: alpha * 0.86,
        color,
        shape: particleShape,
      })
    } else if (node.preset === 'dust') {
      particles.push({
        x,
        y,
        radius,
        sizeX,
        sizeY,
        angle: renderAngle,
        alpha: alpha * 0.7,
        color,
        shape: particleShape,
      })
    } else {
      particles.push({
        x,
        y,
        radius,
        sizeX,
        sizeY,
        angle: renderAngle,
        alpha,
        color,
        shape: particleShape,
      })
    }
  }

  return particles
}

type ComposerParticleBlocker = {
  x: number
  y: number
  width: number
  height: number
  polygon: Array<{ x: number; y: number }>
  occluder: boolean
  surface: boolean
}

function buildComposerParticleBlockers(
  images: Map<string, HTMLImageElement>,
  nodes: SceneNode[],
  width: number,
  height: number,
  timeSeconds: number,
): ComposerParticleBlocker[] {
  const canvas = document.createElement('canvas')
  canvas.width = Math.max(1, width)
  canvas.height = Math.max(1, height)
  const context = canvas.getContext('2d')
  if (!context) {
    return []
  }

  const blockers: ComposerParticleBlocker[] = []
  for (const node of nodes) {
    if (node.kind !== 'sprite' || !node.enabled) {
      continue
    }
    if (!node.particle_occluder && !node.particle_surface) {
      continue
    }
    const image = images.get(node.image_key)
    if (!image) {
      continue
    }
    const layout = resolveComposerSpriteLayout(context, image, node, timeSeconds)
    const region = resolveSpriteParticleRegionRect(layout, node.particle_region ?? null)
    blockers.push({
      x: region.x,
      y: region.y,
      width: region.width,
      height: region.height,
      polygon: [],
      occluder: Boolean(node.particle_occluder),
      surface: Boolean(node.particle_surface),
    })
  }
  for (const node of nodes) {
    if (node.kind !== 'particle_area' || !node.enabled) {
      continue
    }
    blockers.push({
      x: width * node.region.x,
      y: height * node.region.y,
      width: width * node.region.width,
      height: height * node.region.height,
      polygon: (node.shape ?? 'rect') === 'polygon'
        ? (node.points ?? []).map((point) => ({ x: width * point.x, y: height * point.y }))
        : [],
      occluder: node.occluder,
      surface: node.surface,
    })
  }
  return blockers
}

function isComposerParticleOccluded(
  blockers: ComposerParticleBlocker[],
  x: number,
  y: number,
  radius: number,
) {
  return blockers.some((blocker) => blocker.occluder
    && composerBlockerContains(blocker, x, y, radius))
}

function isComposerParticleSegmentOccluded(
  blockers: ComposerParticleBlocker[],
  start: { x: number; y: number },
  end: { x: number; y: number },
  thicknessRadius: number,
) {
  return blockers.some((blocker) => blocker.occluder
    && composerBlockerContainsSegment(blocker, start, end, thicknessRadius))
}

function resolveComposerParticleCollision(
  node: Extract<SceneNode, { kind: 'emitter' }>,
  blockers: ComposerParticleBlocker[],
  x: number,
  y: number,
  radius: number,
) {
  const surface = blockers
    .map((blocker) => ({ blocker, surfaceY: composerBlockerSurfaceY(blocker, x) }))
    .filter((entry): entry is { blocker: ComposerParticleBlocker; surfaceY: number } => entry.blocker.surface && entry.surfaceY !== null && y >= entry.surfaceY && y <= entry.blocker.y + entry.blocker.height)
    .sort((left, right) => left.surfaceY - right.surfaceY)[0]
  if (!surface) {
    return null
  }
  if (node.preset === 'snow' || node.preset === 'dust') {
    return surface.surfaceY - radius
  }
  return 'discard'
}

function composerBlockerContains(blocker: ComposerParticleBlocker, x: number, y: number, radius: number) {
  if (blocker.polygon.length >= 3) {
    return polygonIntersectsCircle(blocker.polygon, x, y, radius)
  }
  return x >= blocker.x
    && x <= blocker.x + blocker.width
    && y + radius >= blocker.y
    && y - radius <= blocker.y + blocker.height
}

function composerBlockerContainsSegment(
  blocker: ComposerParticleBlocker,
  start: { x: number; y: number },
  end: { x: number; y: number },
  thicknessRadius: number,
) {
  if (blocker.polygon.length >= 3) {
    return polygonIntersectsSegment(blocker.polygon, start, end, thicknessRadius)
  }
  const left = blocker.x - thicknessRadius
  const right = blocker.x + blocker.width + thicknessRadius
  const top = blocker.y - thicknessRadius
  const bottom = blocker.y + blocker.height + thicknessRadius
  return pointInRect(start.x, start.y, { x: left, y: top, width: right - left, height: bottom - top })
    || pointInRect(end.x, end.y, { x: left, y: top, width: right - left, height: bottom - top })
    || segmentsIntersect(start, end, { x: left, y: top }, { x: right, y: top })
    || segmentsIntersect(start, end, { x: right, y: top }, { x: right, y: bottom })
    || segmentsIntersect(start, end, { x: right, y: bottom }, { x: left, y: bottom })
    || segmentsIntersect(start, end, { x: left, y: bottom }, { x: left, y: top })
}

function composerBlockerSurfaceY(blocker: ComposerParticleBlocker, x: number) {
  if (blocker.polygon.length >= 3) {
    return polygonSurfaceY(blocker.polygon, x)
  }
  return x >= blocker.x && x <= blocker.x + blocker.width ? blocker.y : null
}

function pointInPolygon(points: Array<{ x: number; y: number }>, x: number, y: number) {
  let inside = false
  let previous = points.length - 1
  for (let current = 0; current < points.length; current += 1) {
    const a = points[current]
    const b = points[previous]
    const intersects = ((a.y > y) !== (b.y > y))
      && (x < (((b.x - a.x) * (y - a.y)) / Math.max(Math.abs(b.y - a.y), Number.EPSILON)) + a.x)
    if (intersects) {
      inside = !inside
    }
    previous = current
  }
  return inside
}

function polygonIntersectsCircle(
  points: Array<{ x: number; y: number }>,
  x: number,
  y: number,
  radius: number,
) {
  if (pointInPolygon(points, x, y)) {
    return true
  }
  const radiusSq = radius * radius
  for (let index = 0; index < points.length; index += 1) {
    const a = points[index]
    const b = points[(index + 1) % points.length]
    if (distanceSqToSegment({ x, y }, a, b) <= radiusSq) {
      return true
    }
  }
  return false
}

function polygonIntersectsSegment(
  points: Array<{ x: number; y: number }>,
  start: { x: number; y: number },
  end: { x: number; y: number },
  thicknessRadius: number,
) {
  if (pointInPolygon(points, start.x, start.y) || pointInPolygon(points, end.x, end.y)) {
    return true
  }
  const thicknessSq = thicknessRadius * thicknessRadius
  for (let index = 0; index < points.length; index += 1) {
    const a = points[index]
    const b = points[(index + 1) % points.length]
    if (segmentsIntersect(start, end, a, b)) {
      return true
    }
    if (distanceSqBetweenSegments(start, end, a, b) <= thicknessSq) {
      return true
    }
  }
  return false
}

function orientation2d(a: { x: number; y: number }, b: { x: number; y: number }, c: { x: number; y: number }) {
  return ((b.y - a.y) * (c.x - b.x)) - ((b.x - a.x) * (c.y - b.y))
}

function onSegment2d(a: { x: number; y: number }, b: { x: number; y: number }, c: { x: number; y: number }) {
  return b.x >= Math.min(a.x, c.x)
    && b.x <= Math.max(a.x, c.x)
    && b.y >= Math.min(a.y, c.y)
    && b.y <= Math.max(a.y, c.y)
}

function segmentsIntersect(
  p1: { x: number; y: number },
  q1: { x: number; y: number },
  p2: { x: number; y: number },
  q2: { x: number; y: number },
) {
  const o1 = orientation2d(p1, q1, p2)
  const o2 = orientation2d(p1, q1, q2)
  const o3 = orientation2d(p2, q2, p1)
  const o4 = orientation2d(p2, q2, q1)

  if (((o1 > 0 && o2 < 0) || (o1 < 0 && o2 > 0))
    && ((o3 > 0 && o4 < 0) || (o3 < 0 && o4 > 0))) {
    return true
  }
  if (Math.abs(o1) <= Number.EPSILON && onSegment2d(p1, p2, q1)) return true
  if (Math.abs(o2) <= Number.EPSILON && onSegment2d(p1, q2, q1)) return true
  if (Math.abs(o3) <= Number.EPSILON && onSegment2d(p2, p1, q2)) return true
  if (Math.abs(o4) <= Number.EPSILON && onSegment2d(p2, q1, q2)) return true
  return false
}

function distanceSqBetweenSegments(
  a1: { x: number; y: number },
  a2: { x: number; y: number },
  b1: { x: number; y: number },
  b2: { x: number; y: number },
) {
  const d1 = distanceSqToSegment(a1, b1, b2)
  const d2 = distanceSqToSegment(a2, b1, b2)
  const d3 = distanceSqToSegment(b1, a1, a2)
  const d4 = distanceSqToSegment(b2, a1, a2)
  return Math.min(d1, d2, d3, d4)
}

function distanceSqToSegment(
  point: { x: number; y: number },
  a: { x: number; y: number },
  b: { x: number; y: number },
) {
  const abX = b.x - a.x
  const abY = b.y - a.y
  const apX = point.x - a.x
  const apY = point.y - a.y
  const abLenSq = (abX * abX) + (abY * abY)
  if (abLenSq <= Number.EPSILON) {
    const dx = point.x - a.x
    const dy = point.y - a.y
    return (dx * dx) + (dy * dy)
  }
  const t = Math.max(0, Math.min(1, ((apX * abX) + (apY * abY)) / abLenSq))
  const closestX = a.x + (abX * t)
  const closestY = a.y + (abY * t)
  const dx = point.x - closestX
  const dy = point.y - closestY
  return (dx * dx) + (dy * dy)
}

function polygonSurfaceY(points: Array<{ x: number; y: number }>, x: number) {
  const hits: number[] = []
  for (let index = 0; index < points.length; index += 1) {
    const a = points[index]
    const b = points[(index + 1) % points.length]
    const minX = Math.min(a.x, b.x)
    const maxX = Math.max(a.x, b.x)
    if (x < minX || x > maxX) {
      continue
    }
    if (Math.abs(b.x - a.x) <= Number.EPSILON) {
      hits.push(Math.min(a.y, b.y))
      continue
    }
    const t = (x - a.x) / (b.x - a.x)
    if (t < 0 || t > 1) {
      continue
    }
    hits.push(a.y + ((b.y - a.y) * t))
  }
  return hits.length > 0 ? Math.min(...hits) : null
}

function sampleEmitterPosition(
  node: Extract<SceneNode, { kind: 'emitter' }>,
  width: number,
  height: number,
  originX: number,
  originY: number,
  random: () => number,
) {
  const shape = resolveEmitterShape(node)
  if (shape === 'point') {
    return { x: originX, y: originY }
  }
  if (shape === 'box') {
    return {
      x: originX + ((random() - 0.5) * resolveEmitterRegionWidth(node) * width),
      y: originY + ((random() - 0.5) * resolveEmitterRegionHeight(node) * height),
    }
  }
  if (shape === 'line') {
    const length = resolveEmitterLineLength(node) * width
    const angle = (resolveEmitterLineAngleDeg(node) * Math.PI) / 180
    const offset = (random() - 0.5) * length
    return {
      x: originX + Math.cos(angle) * offset,
      y: originY + Math.sin(angle) * offset,
    }
  }
  const radius = resolveEmitterRegionRadius(node) * Math.min(width, height)
  const theta = random() * Math.PI * 2
  const distance = Math.sqrt(random()) * radius
  return {
    x: originX + Math.cos(theta) * distance,
    y: originY + Math.sin(theta) * distance,
  }
}

function evaluateScalarCurve(curve: SceneCurvePoint[], x: number) {
  const points = resolveScalarCurve(curve, curve)
  if (x <= points[0].x) {
    return points[0].y
  }
  for (let index = 1; index < points.length; index += 1) {
    const left = points[index - 1]
    const right = points[index]
    if (x <= right.x) {
      const t = (x - left.x) / Math.max(right.x - left.x, 0.0001)
      return left.y + ((right.y - left.y) * t)
    }
  }
  return points[points.length - 1].y
}

function evaluateColorCurve(curve: SceneColorStop[], x: number, fallback: string) {
  const stops = resolveColorCurve(curve, [{ x: 0, color_hex: fallback }, { x: 1, color_hex: fallback }])
  if (x <= stops[0].x) {
    return stops[0].color_hex
  }
  for (let index = 1; index < stops.length; index += 1) {
    const left = stops[index - 1]
    const right = stops[index]
    if (x <= right.x) {
      const t = (x - left.x) / Math.max(right.x - left.x, 0.0001)
      const from = parseColorHex(left.color_hex)
      const to = parseColorHex(right.color_hex)
      const mix = (start: number, end: number) => Math.round(start + ((end - start) * t))
      return `rgb(${mix(from.red, to.red)}, ${mix(from.green, to.green)}, ${mix(from.blue, to.blue)})`
    }
  }
  return stops[stops.length - 1].color_hex
}

function smoothstep(edge0: number, edge1: number, x: number) {
  const t = Math.min(1, Math.max(0, (x - edge0) / Math.max(edge1 - edge0, 0.00001)))
  return t * t * (3 - 2 * t)
}

function stablePreviewSeed(index: number, node: Extract<SceneNode, { kind: 'emitter' }>) {
  let hash = 2166136261 ^ index
  const text = `${node.id}:${node.preset}:${node.emission_rate}:${node.speed}:${node.spread}:${resolveEmitterOriginX(node)}:${resolveEmitterOriginY(node)}:${resolveEmitterDirectionDeg(node)}:${resolveEmitterColorHex(node)}`
  for (let i = 0; i < text.length; i += 1) {
    hash ^= text.charCodeAt(i)
    hash = Math.imul(hash, 16777619)
  }
  return hash >>> 0
}

function nextPreviewSeed(seed: number) {
  return (Math.imul(seed, 1664525) + 1013904223) >>> 0
}

function nextPreviewRandom(seed: number) {
  return seed / 0xffffffff
}

function assetPreviewDataUri(asset: AssetMetadata) {
  return `data:image/svg+xml;utf8,${encodeURIComponent(assetPreviewSvg(asset))}`
}

function assetPreviewSvg(asset: AssetMetadata) {
  switch (asset.id) {
    case 'demo.neon-grid':
      return `
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 160">
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
          <rect x="0" y="0" width="320" height="160" fill="url(#grid)" transform="skewY(-8)"/>
          <rect width="320" height="160" fill="none" stroke="#8cf3dd" stroke-opacity="0.18"/>
        </svg>
      `
    case 'demo.ember-scan':
      return `
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 160">
          <defs>
            <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
              <stop offset="0%" stop-color="#140907"/>
              <stop offset="100%" stop-color="#3b1207"/>
            </linearGradient>
          </defs>
          <rect width="320" height="160" fill="url(#bg)"/>
          <rect x="178" width="28" height="160" fill="#ff7b22" fill-opacity="0.75"/>
          <rect x="184" width="10" height="160" fill="#ffd6ad" fill-opacity="0.92"/>
          <g stroke="#ff9447" stroke-opacity="0.2">
            <path d="M0 24 H320"/><path d="M0 44 H320"/><path d="M0 64 H320"/><path d="M0 84 H320"/><path d="M0 104 H320"/><path d="M0 124 H320"/>
          </g>
          <circle cx="160" cy="84" r="78" fill="#b93818" fill-opacity="0.08"/>
        </svg>
      `
    case 'demo.tide-pulse':
      return `
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 160">
          <defs>
            <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
              <stop offset="0%" stop-color="#06111d"/>
              <stop offset="100%" stop-color="#164a73"/>
            </linearGradient>
          </defs>
          <rect width="320" height="160" fill="url(#bg)"/>
          <path d="M0 112 C40 92 70 72 110 80 C150 88 166 120 206 118 C246 116 270 78 320 58 L320 160 L0 160 Z" fill="#3786d8" fill-opacity="0.7"/>
          <path d="M0 130 C38 120 78 96 124 102 C170 108 196 138 236 130 C276 122 292 102 320 90 L320 160 L0 160 Z" fill="#86dfff" fill-opacity="0.58"/>
          <circle cx="88" cy="46" r="24" fill="#92e9ff" fill-opacity="0.28"/>
        </svg>
      `
    case 'demo.sunset-stripes':
      return imagePreviewSvg('cover')
    default:
      if (asset.kind === 'web') {
        return `
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 160">
            <defs>
              <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
                <stop offset="0%" stop-color="#08111c"/>
                <stop offset="100%" stop-color="#1d3555"/>
              </linearGradient>
            </defs>
            <rect width="320" height="160" fill="url(#bg)"/>
            <rect x="38" y="26" width="244" height="108" rx="12" fill="#0a1526" stroke="#9ed0ff" stroke-opacity="0.25"/>
            <rect x="56" y="46" width="208" height="12" rx="6" fill="#7fc7ff" fill-opacity="0.45"/>
            <rect x="56" y="68" width="156" height="10" rx="5" fill="#ffffff" fill-opacity="0.15"/>
            <rect x="56" y="86" width="184" height="10" rx="5" fill="#ffffff" fill-opacity="0.12"/>
            <rect x="56" y="104" width="126" height="10" rx="5" fill="#ffffff" fill-opacity="0.1"/>
          </svg>
        `
      }
      if (asset.kind === 'scene') {
        return `
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 160">
            <defs>
              <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
                <stop offset="0%" stop-color="#140b1d"/>
                <stop offset="100%" stop-color="#2b1d48"/>
              </linearGradient>
            </defs>
            <rect width="320" height="160" fill="url(#bg)"/>
            <circle cx="92" cy="72" r="46" fill="#9f7aea" fill-opacity="0.2"/>
            <circle cx="198" cy="60" r="30" fill="#60a5fa" fill-opacity="0.22"/>
            <circle cx="244" cy="108" r="38" fill="#34d399" fill-opacity="0.18"/>
            <path d="M20 130 C80 80 140 150 200 100 C240 68 276 90 300 72" fill="none" stroke="#f8fafc" stroke-opacity="0.22" stroke-width="4"/>
          </svg>
        `
      }
      if (asset.kind === 'video') {
        return `
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 160">
            <defs>
              <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
                <stop offset="0%" stop-color="#101418"/>
                <stop offset="100%" stop-color="#24313b"/>
              </linearGradient>
            </defs>
            <rect width="320" height="160" fill="url(#bg)"/>
            <rect x="52" y="28" width="216" height="104" rx="14" fill="#091018" stroke="#ffffff" stroke-opacity="0.12"/>
            <polygon points="144,64 144,96 176,80" fill="#f8fafc" fill-opacity="0.8"/>
          </svg>
        `
      }
      return `
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 160">
          <rect width="320" height="160" fill="#0b1320"/>
          <rect x="1" y="1" width="318" height="158" fill="none" stroke="#ffffff" stroke-opacity="0.08"/>
        </svg>
      `
  }
}

function imagePreviewSvg(mode: 'cover' | 'contain' | 'center') {
  if (mode === 'contain') {
    return `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 160">
        <rect width="320" height="160" fill="#0a1120"/>
        ${sunsetArtwork(32, 24, 256, 112)}
      </svg>
    `
  }

  if (mode === 'center') {
    return `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 160">
        <rect width="320" height="160" fill="#0a1120"/>
        ${sunsetArtwork(84, 40, 152, 76)}
      </svg>
    `
  }

  return `
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 160">
      ${sunsetArtwork(0, 0, 320, 160)}
    </svg>
  `
}

function sunsetArtwork(x: number, y: number, width: number, height: number) {
  const midX = x + width / 2
  const midY = y + height / 2
  return `
    <defs>
      <linearGradient id="sunset" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0%" stop-color="#f7efe3"/>
        <stop offset="4%" stop-color="#f7efe3"/>
        <stop offset="4%" stop-color="#6aa2da"/>
        <stop offset="42%" stop-color="#2f6cae"/>
        <stop offset="42%" stop-color="#ffffff"/>
        <stop offset="46%" stop-color="#ffffff"/>
        <stop offset="46%" stop-color="#ea9652"/>
        <stop offset="96%" stop-color="#f1cf7a"/>
        <stop offset="96%" stop-color="#f7efe3"/>
        <stop offset="100%" stop-color="#f7efe3"/>
      </linearGradient>
    </defs>
    <rect x="${x}" y="${y}" width="${width}" height="${height}" fill="url(#sunset)"/>
    <rect x="${x}" y="${y}" width="${width}" height="${height}" fill="none" stroke="#f7efe3" stroke-width="6"/>
    <rect x="${midX - 4}" y="${y}" width="8" height="${height}" fill="#ffffff"/>
    <rect x="${x}" y="${midY - 4}" width="${width}" height="8" fill="#ffffff"/>
  `
}
