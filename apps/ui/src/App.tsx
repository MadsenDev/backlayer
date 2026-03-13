import { motion } from 'framer-motion'
import { startTransition, useDeferredValue, useEffect, useRef, useState } from 'react'
import type { ChangeEvent, ReactNode } from 'react'
import {
  FiActivity,
  FiAlertCircle,
  FiCpu,
  FiDisc,
  FiDownload,
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
  dataUrl: string
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
type AssetContextMenuState = {
  assetId: string
  x: number
  y: number
}

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
  const [importPath, setImportPath] = useState('')
  const [suggestedWorkshopPaths, setSuggestedWorkshopPaths] = useState<string[]>([])
  const [importResult, setImportResult] = useState<AssetMetadata[]>([])
  const [assetSearch, setAssetSearch] = useState('')
  const [filtersOpen, setFiltersOpen] = useState(false)
  const [sourceFilter, setSourceFilter] = useState<'all' | 'native' | 'wallpaper_engine_import'>('all')
  const [kindFilter, setKindFilter] = useState<'all' | 'image' | 'video' | 'shader' | 'scene' | 'web'>('all')
  const [compatibilityFilter, setCompatibilityFilter] = useState<'all' | 'supported' | 'partial' | 'unsupported'>('all')
  const [composerOpen, setComposerOpen] = useState(false)
  const [composerName, setComposerName] = useState('')
  const [composerBaseAssetId, setComposerBaseAssetId] = useState<string | null>(null)
  const [composerUploadImage, setComposerUploadImage] = useState<ComposerUploadImage | null>(null)
  const [composerExtraImages, setComposerExtraImages] = useState<ComposerUploadImage[]>([])
  const [composerEditingAssetId, setComposerEditingAssetId] = useState<string | null>(null)
  const [composerNodes, setComposerNodes] = useState<SceneNode[]>(() => buildDefaultComposerNodes())
  const [selectedComposerNodeId, setSelectedComposerNodeId] = useState<string | null>(null)
  const [composerLeftTab, setComposerLeftTab] = useState<ComposerLeftTab>('layers')
  const [particleEditorNodeId, setParticleEditorNodeId] = useState<string | null>(null)
  const [notice, setNotice] = useState<UINotice | null>(null)
  const [assetContextMenu, setAssetContextMenu] = useState<AssetContextMenuState | null>(null)
  const [themePreference, setThemePreference] = useState<ThemePreference>(() => readThemePreference())
  const [systemTheme, setSystemTheme] = useState<ActiveTheme>(() => detectSystemTheme())
  const composerFileInputRef = useRef<HTMLInputElement | null>(null)
  const composerSpriteFileInputRef = useRef<HTMLInputElement | null>(null)

  async function refreshSnapshot() {
    const nextSnapshot = await fetchRuntimeSnapshot()
    startTransition(() => {
      setSnapshot(nextSnapshot)
      setLoading(false)
    })
  }

  useEffect(() => {
    let cancelled = false

    void fetchRuntimeSnapshot().then((nextSnapshot) => {
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
    if (snapshot?.source !== 'tauri' && snapshot?.source !== 'tauri_disconnected') {
      return
    }

    const interval = window.setInterval(() => {
      void refreshSnapshot().catch(() => {})
    }, 2000)

    return () => {
      window.clearInterval(interval)
    }
  }, [snapshot?.source])

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
  const filteredAssets = assets.filter((asset) => {
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
  })
  const selectedMonitor =
    monitors.find((monitor) => monitor.id === selectedMonitorId)
    ?? monitors.find((monitor) => monitor.focused)
    ?? monitors[0]
    ?? null
  const selectedAssignment = selectedMonitor
    ? assignments.find(
        ({ monitor_id }) =>
          monitor_id === selectedMonitor.id || monitor_id === selectedMonitor.output_name,
      ) ?? null
    : null
  const selectedAsset =
    assets.find((asset) => asset.id === selectedAssetId)
    ?? selectedAssignment?.wallpaper
    ?? assets[0]
    ?? null
  const composerBaseAsset = composerBaseAssetId
    ? assets.find((asset) => asset.id === composerBaseAssetId) ?? null
    : null
  const composerSourceName = composerUploadImage?.name ?? composerBaseAsset?.name ?? null
  const selectedComposerNode = composerNodes.find((node) => node.id === selectedComposerNodeId) ?? composerNodes[0] ?? null
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
      await refreshSnapshot()
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
      await refreshSnapshot()
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
      await refreshSnapshot()
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
      await refreshSnapshot()
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
      await refreshSnapshot()
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
      await refreshSnapshot()
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
      await refreshSnapshot()
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
      base_asset_id: composerUploadImage ? null : composerBaseAsset?.id ?? null,
      base_image_data_url: composerUploadImage?.dataUrl ?? null,
      base_image_filename: composerUploadImage?.filename ?? null,
      extra_images: composerExtraImages.map((image) => ({
        key: image.key,
        data_url: image.dataUrl,
        filename: image.filename,
      })) satisfies CreateSceneImageSourceRequest[],
      nodes: composerNodes,
    }

    setBusy('create-scene')
    setUiError(null)
    try {
      const asset = await createSceneAsset(request)
      await refreshSnapshot()
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

  function openComposer(initialAsset: AssetMetadata | null = selectedAsset) {
    setUiError(null)
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
      setComposerName(`${initialAsset.name} Scene`)
    } else {
      setComposerBaseAssetId(null)
      setComposerName('')
    }
  }

function closeComposer() {
    setComposerOpen(false)
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
    setBusy(`load-scene:${asset.id}`)
    setUiError(null)
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
        dataUrl: image.data_url,
      })

      setSelectedAssetId(asset.id)
      setComposerOpen(true)
      setComposerEditingAssetId(asset.id)
      setComposerBaseAssetId(null)
      setComposerUploadImage(toComposerImage(baseImage))
      setComposerExtraImages(editable.images.filter((image) => image.key !== 'base').map(toComposerImage))
      setComposerNodes(editable.document.nodes)
      setSelectedComposerNodeId(editable.document.nodes[0]?.id ?? null)
      setComposerName(editable.asset.name)
      setComposerLeftTab('layers')
    } catch (error) {
      setUiError(error instanceof Error ? error.message : String(error))
    } finally {
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

  async function handleComposerFilePicked(event: ChangeEvent<HTMLInputElement>) {
    const file = event.currentTarget.files?.[0]
    if (!file) {
      return
    }

    const dataUrl = await new Promise<string>((resolve, reject) => {
      const reader = new FileReader()
      reader.onload = () => resolve(String(reader.result ?? ''))
      reader.onerror = () => reject(new Error('Failed to read image file.'))
      reader.readAsDataURL(file)
    }).catch((error) => {
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

    const dataUrl = await new Promise<string>((resolve, reject) => {
      const reader = new FileReader()
      reader.onload = () => resolve(String(reader.result ?? ''))
      reader.onerror = () => reject(new Error('Failed to read sprite image file.'))
      reader.readAsDataURL(file)
    }).catch((error) => {
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
                onClick={() => openComposer()}
                type="button"
              >
                <FiImage className="size-4" />
                Create Scene
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
                  disabled={busy === 'create-scene' || (!composerBaseAsset && !composerUploadImage)}
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
                    {selectedComposerNode ? <span className="control-chip">{selectedComposerNode.kind}</span> : null}
                    {composerSourceName ? <span className="control-chip">{composerSourceName}</span> : null}
                  </div>
                </div>
                <div className="h-full min-h-0 p-4">
                  {composerBaseAsset || composerUploadImage ? (
                    <ComposerEnginePreview
                      asset={composerBaseAsset}
                      nodes={composerNodes}
                      extraImages={composerExtraImages}
                      uploadedImageDataUrl={composerUploadImage?.dataUrl ?? null}
                      selectedEmitter={selectedComposerNode?.kind === 'emitter' ? selectedComposerNode : null}
                      selectedNode={selectedComposerNode}
                      onPlaceEmitter={(originX, originY) => {
                        if (!selectedComposerNode || selectedComposerNode.kind !== 'emitter') {
                          return
                        }
                        const nextNode: SceneEmitterNode = {
                          ...selectedComposerNode,
                          origin_x: originX,
                          origin_y: originY,
                        }
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
                      node={selectedComposerNode}
                      onOpenParticleEditor={(nodeId) => setParticleEditorNodeId(nodeId)}
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
    size_curve: defaultEmitterSizeCurve(preset),
    alpha_curve: defaultEmitterAlphaCurve(preset),
    color_curve: defaultEmitterColorCurve(preset),
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
  return (
    asset.source_kind === 'wallpaper_engine_import'
    || (asset.source_kind === 'native'
      && (asset.entrypoint.includes('/.config/backlayer/assets/')
        || asset.preview_image?.includes('/.config/backlayer/assets/')))
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
    ? 'Imported Wallpaper Engine video item. Import works, but video playback is still unfinished.'
    : 'Video wallpaper support is still planned, not implemented.'
}

function importRuntimeNote(kind: AssetMetadata['kind']) {
  switch (kind) {
    case 'scene':
      return 'Imported Wallpaper Engine scene items are currently limited to static scene-image extraction or preview fallback. Full scene playback and animation are not implemented yet.'
    case 'web':
      return 'Imported Wallpaper Engine web items currently render a narrow static subset: a local HTML image, a parsed background color, or the preview fallback. Real browser-backed wallpaper playback is not implemented yet.'
    case 'video':
      return 'Imported Wallpaper Engine video items currently use a static preview fallback. Real video playback and animation are not implemented yet.'
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

function AssetPreview({ asset }: { asset: AssetMetadata }) {
  const [previewUrl, setPreviewUrl] = useState<string | null>(null)
  const previewPath = asset.preview_image ?? (asset.kind === 'image' ? asset.entrypoint : null)

  useEffect(() => {
    let cancelled = false

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
  }, [previewPath])

  return (
    <div
      className="h-full w-full bg-cover bg-center"
      style={{ backgroundImage: `url("${previewUrl ?? assetPreviewDataUri(asset)}")` }}
    />
  )
}

function AssetCard({
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
}

function ComposerEnginePreview({
  asset,
  extraImages,
  onPlaceEmitter,
  selectedNode,
  selectedEmitter,
  uploadedImageDataUrl,
  nodes,
  saving,
}: {
  asset: AssetMetadata | null
  extraImages: ComposerUploadImage[]
  onPlaceEmitter: (originX: number, originY: number) => void
  selectedNode: SceneNode | null
  selectedEmitter: SceneEmitterNode | null
  uploadedImageDataUrl: string | null
  nodes: SceneNode[]
  saving: boolean
}) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null)
  const [previewUrl, setPreviewUrl] = useState<string | null>(null)
  const [previewSize, setPreviewSize] = useState({ width: 1280, height: 720 })
  const [loading, setLoading] = useState(true)
  const [loadError, setLoadError] = useState<string | null>(null)
  const previewPath = uploadedImageDataUrl
    ? null
    : asset?.preview_image ?? (asset?.kind === 'image' ? asset.entrypoint : null)

  useEffect(() => {
    let cancelled = false

    if (uploadedImageDataUrl) {
      setPreviewUrl(uploadedImageDataUrl)
      setLoading(false)
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
          setLoading(false)
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
  }, [previewPath, uploadedImageDataUrl])

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas || !previewUrl) {
      return
    }

    const context = canvas.getContext('2d')
    if (!context) {
      return
    }

    let disposed = false
    let frameHandle = 0
    const imageMap = new Map<string, HTMLImageElement>()
    const loadImage = (key: string, src: string) =>
      new Promise<void>((resolve, reject) => {
        const image = new Image()
        image.crossOrigin = 'anonymous'
        image.onload = () => {
          imageMap.set(key, image)
          resolve()
        }
        image.onerror = () => reject(new Error(`Failed to load image ${key}`))
        image.src = src
      })

    Promise.all([
      loadImage('base', previewUrl),
      ...extraImages.map((image) => loadImage(image.key, image.dataUrl)),
    ])
      .then(() => {
        const startedAt = performance.now()
        const render = (now: number) => {
          if (disposed) {
            return
          }
          const baseImage = imageMap.get('base')
          if (baseImage && (canvas.width !== baseImage.naturalWidth || canvas.height !== baseImage.naturalHeight)) {
            canvas.width = Math.max(1, baseImage.naturalWidth)
            canvas.height = Math.max(1, baseImage.naturalHeight)
            setPreviewSize({
              width: Math.max(1, baseImage.naturalWidth),
              height: Math.max(1, baseImage.naturalHeight),
            })
          }
          drawComposerSceneFrame(context, imageMap, nodes, (now - startedAt) / 1000)
          frameHandle = window.requestAnimationFrame(render)
        }
        render(startedAt)
      })
      .catch(() => {
        if (!disposed) {
          setLoadError('Failed to decode scene source image for preview.')
        }
      })

    return () => {
      disposed = true
      window.cancelAnimationFrame(frameHandle)
    }
  }, [extraImages, nodes, previewUrl])

  return (
    <div className="flex h-full min-h-[360px] items-center justify-center">
      <div
        className={[
          'relative w-full overflow-hidden border border-white/10 bg-slate-950/90',
          selectedEmitter ? 'cursor-crosshair' : '',
        ].join(' ')}
        onPointerDown={(event) => {
          if (!selectedEmitter) {
            return
          }
          const target = event.currentTarget
          const rect = target.getBoundingClientRect()
          const originX = clamp01((event.clientX - rect.left) / Math.max(rect.width, 1))
          const originY = clamp01((event.clientY - rect.top) / Math.max(rect.height, 1))
          onPlaceEmitter(originX, originY)
        }}
        onPointerMove={(event) => {
          if (!selectedEmitter || (event.buttons & 1) === 0) {
            return
          }
          const target = event.currentTarget
          const rect = target.getBoundingClientRect()
          const originX = clamp01((event.clientX - rect.left) / Math.max(rect.width, 1))
          const originY = clamp01((event.clientY - rect.top) / Math.max(rect.height, 1))
          onPlaceEmitter(originX, originY)
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
              <div className="mt-1 uppercase tracking-[0.16em] text-slate-400">{selectedNode.kind}</div>
            </div>
          </div>
        ) : null}
        {selectedEmitter ? (
          <>
            <EmitterViewportOverlay emitter={selectedEmitter} />
            <div className="pointer-events-none absolute left-4 top-20">
              <div className="surface-panel px-3 py-2 text-xs text-slate-200">
                Click or drag in the viewport to place the selected emitter.
              </div>
            </div>
          </>
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

function EmitterViewportOverlay({ emitter }: { emitter: SceneEmitterNode }) {
  const left = `${resolveEmitterOriginX(emitter) * 100}%`
  const top = `${resolveEmitterOriginY(emitter) * 100}%`
  const shape = resolveEmitterShape(emitter)
  const common = 'pointer-events-none absolute border border-cyan-200/70 bg-cyan-200/5'
  return (
    <>
      {shape === 'box' ? (
        <div
          className={common}
          style={{
            left,
            top,
            width: `${resolveEmitterRegionWidth(emitter) * 100}%`,
            height: `${resolveEmitterRegionHeight(emitter) * 100}%`,
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
            width: `${resolveEmitterRegionRadius(emitter) * 200}%`,
            height: `${resolveEmitterRegionRadius(emitter) * 200}%`,
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
            width: `${resolveEmitterLineLength(emitter) * 100}%`,
            transform: `translate(-50%, -50%) rotate(${resolveEmitterLineAngleDeg(emitter)}deg)`,
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
        className="pointer-events-none absolute h-[2px] origin-left bg-cyan-200/85"
        style={{
          left,
          top,
          width: '56px',
          transform: `translateY(-50%) rotate(${resolveEmitterDirectionDeg(emitter)}deg)`,
        }}
      />
    </>
  )
}

function ComposerNodeInspector({
  availableImageKeys,
  node,
  onOpenParticleEditor,
  onChange,
}: {
  availableImageKeys: Array<{ key: string; label: string }>
  node: SceneNode
  onOpenParticleEditor: (nodeId: string) => void
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
      {node.kind === 'sprite' ? <ComposerSpriteInspector availableImageKeys={availableImageKeys} node={node} onChange={onChange} /> : null}
      {node.kind === 'effect' ? <ComposerEffectInspector node={node} onChange={onChange} /> : null}
      {node.kind === 'emitter' ? <ComposerEmitterInspector availableImageKeys={availableImageKeys} node={node} onChange={onChange} onOpenParticleEditor={onOpenParticleEditor} /> : null}
    </div>
  )
}

function ComposerSpriteInspector({
  availableImageKeys,
  node,
  onChange,
}: {
  availableImageKeys: Array<{ key: string; label: string }>
  node: SceneSpriteNode
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
  node,
  onClose,
  onSave,
}: {
  availableImageKeys: Array<{ key: string; label: string }>
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
              <ParticlePreviewPanel availableImageKeys={availableImageKeys} node={previewNode} />
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
  availableImageKeys,
  node,
}: {
  availableImageKeys: Array<{ key: string; label: string }>
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
    const render = (now: number) => {
      if (disposed) {
        return
      }
      canvas.width = 960
      canvas.height = 540
      context.clearRect(0, 0, canvas.width, canvas.height)
      const gradient = context.createLinearGradient(0, 0, canvas.width, canvas.height)
      gradient.addColorStop(0, '#040810')
      gradient.addColorStop(1, '#0b1522')
      context.fillStyle = gradient
      context.fillRect(0, 0, canvas.width, canvas.height)
      drawPreviewGrid(context, canvas.width, canvas.height)
      drawComposerEmitterNode(context, new Map<string, HTMLImageElement>(), node, now / 1000)
      frame = window.requestAnimationFrame(render)
    }
    frame = window.requestAnimationFrame(render)
    return () => {
      disposed = true
      window.cancelAnimationFrame(frame)
    }
  }, [availableImageKeys, node])

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
  timeSeconds: number,
) {
  const width = context.canvas.width
  const height = context.canvas.height
  context.clearRect(0, 0, width, height)
  context.fillStyle = '#05070a'
  context.fillRect(0, 0, width, height)

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
      drawComposerEmitterNode(context, images, node, timeSeconds)
    }
  }
}

function drawComposerSpriteNode(
  context: CanvasRenderingContext2D,
  image: HTMLImageElement,
  node: SceneSpriteNode,
  timeSeconds: number,
) {
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
  const sourceAspect = image.naturalWidth / image.naturalHeight
  const canvasAspect = context.canvas.width / context.canvas.height
  let drawWidth = image.naturalWidth * scale
  let drawHeight = image.naturalHeight * scale

  if (fit === 'cover') {
    if (sourceAspect > canvasAspect) {
      drawHeight = context.canvas.height
      drawWidth = drawHeight * sourceAspect
    } else {
      drawWidth = context.canvas.width
      drawHeight = drawWidth / sourceAspect
    }
  } else if (fit === 'contain') {
    if (sourceAspect > canvasAspect) {
      drawWidth = context.canvas.width
      drawHeight = drawWidth / sourceAspect
    } else {
      drawHeight = context.canvas.height
      drawWidth = drawHeight * sourceAspect
    }
  } else if (fit === 'stretch') {
    drawWidth = context.canvas.width
    drawHeight = context.canvas.height
  }

  const x = (context.canvas.width - drawWidth) / 2 + offsetX
  const y = (context.canvas.height - drawHeight) / 2 + offsetY

  context.save()
  context.globalAlpha = opacity
  context.translate(x + drawWidth / 2, y + drawHeight / 2)
  context.rotate((node.rotation_deg * Math.PI) / 180)
  context.drawImage(image, -drawWidth / 2, -drawHeight / 2, drawWidth, drawHeight)
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
  timeSeconds: number,
) {
  const particles = buildComposerEmitterParticles(node, context.canvas.width, context.canvas.height, timeSeconds)
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
    const y = spawnPosition.y + vy * age + (0.5 * node.gravity_y * age * age)
    const progress = Math.min(1, age / maxLife)
    const sizeScale = evaluateScalarCurve(sizeCurve, progress)
    const alpha = Math.max(0, alphaBase * evaluateScalarCurve(alphaCurve, progress))
    if (alpha <= 0) {
      continue
    }
    const color = evaluateColorCurve(colorCurve, progress, resolveEmitterColorHex(node))
    const particleShape = node.preset === 'rain' && !textured ? 'streak' : textured ? 'texture' : 'circle'

    if (node.preset === 'rain') {
      particles.push({
        x,
        y,
        radius: 0,
        sizeX: size * sizeScale * 1.2,
        sizeY: size * sizeScale * 8.5,
        angle: Math.atan2(vy, vx),
        alpha: alpha * 0.92,
        color,
        shape: particleShape,
      })
    } else if (node.preset === 'snow') {
      particles.push({
        x,
        y,
        radius: size * sizeScale,
        sizeX: size * sizeScale * 2.0,
        sizeY: size * sizeScale * 2.0,
        angle: 0,
        alpha: alpha * 0.86,
        color,
        shape: particleShape,
      })
    } else if (node.preset === 'dust') {
      particles.push({
        x,
        y,
        radius: size * sizeScale,
        sizeX: size * sizeScale * 2.2,
        sizeY: size * sizeScale * 2.2,
        angle: 0,
        alpha: alpha * 0.7,
        color,
        shape: particleShape,
      })
    } else {
      particles.push({
        x,
        y,
        radius: size * sizeScale,
        sizeX: size * sizeScale * 2.0,
        sizeY: size * sizeScale * 2.0,
        angle: 0,
        alpha,
        color,
        shape: particleShape,
      })
    }
  }

  return particles
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
