export type WallpaperKind = 'image' | 'video' | 'shader' | 'scene' | 'web'
export type ImageFitMode = 'cover' | 'contain' | 'stretch' | 'center'
export type AssetSourceKind = 'native' | 'wallpaper_engine_import'
export type CompatibilityStatus = 'supported' | 'partial' | 'unsupported'

export type CompatibilityInfo = {
  status: CompatibilityStatus
  warnings: string[]
}

export type ImportSourceApp = 'wallpaper_engine'

export type ImportMetadata = {
  source_app: ImportSourceApp
  source_path: string
  manifest_path?: string | null
  workshop_id?: string | null
  original_type?: string | null
}

export type AssetMetadata = {
  id: string
  name: string
  kind: WallpaperKind
  animated: boolean
  image_fit?: ImageFitMode | null
  source_kind: AssetSourceKind
  preview_image?: string | null
  compatibility: CompatibilityInfo
  import_metadata?: ImportMetadata | null
  entrypoint: string
}

export type MonitorAssignment = {
  monitor_id: string
  wallpaper: AssetMetadata
  settings: AssignmentSettings
}

export type AssignmentSettings = {
  image_fit?: ImageFitMode | null
}

export type SceneBlendMode = 'alpha' | 'add' | 'screen' | 'multiply'
export type SceneBehaviorKind = 'drift' | 'pulse' | 'orbit'
export type SceneEffectKind = 'glow' | 'vignette' | 'scanlines' | 'fog'
export type SceneEmitterPreset = 'embers' | 'rain' | 'dust' | 'snow'

export type SceneBehavior = {
  kind: SceneBehaviorKind
  speed: number
  amount_x: number
  amount_y: number
  amount: number
  phase: number
}

export type SceneImageSource = {
  key: string
  path: string
}

export type SceneSpriteNode = {
  kind: 'sprite'
  id: string
  name: string
  enabled: boolean
  image_key: string
  fit?: ImageFitMode | null
  blend?: SceneBlendMode | null
  x: number
  y: number
  scale: number
  rotation_deg: number
  opacity: number
  behaviors: SceneBehavior[]
}

export type SceneEffectNode = {
  kind: 'effect'
  id: string
  name: string
  enabled: boolean
  effect: SceneEffectKind
  color_hex?: string | null
  opacity: number
  intensity: number
  speed: number
}

export type SceneEmitterNode = {
  kind: 'emitter'
  id: string
  name: string
  enabled: boolean
  preset: SceneEmitterPreset
  shape?: SceneEmitterShape | null
  origin_x?: number | null
  origin_y?: number | null
  direction_deg?: number | null
  region_width?: number | null
  region_height?: number | null
  region_radius?: number | null
  line_length?: number | null
  line_angle_deg?: number | null
  emission_rate: number
  burst_count: number
  burst_on_start: boolean
  max_particles: number
  opacity: number
  size: number
  speed: number
  min_speed?: number | null
  max_speed?: number | null
  min_life?: number | null
  max_life?: number | null
  spread: number
  gravity_x: number
  gravity_y: number
  drag: number
  color_hex?: string | null
  particle_image_key?: string | null
  size_curve: SceneCurvePoint[]
  alpha_curve: SceneCurvePoint[]
  color_curve: SceneColorStop[]
}

export type SceneEmitterShape = 'point' | 'box' | 'line' | 'circle'
export type SceneCurvePoint = { x: number; y: number }
export type SceneColorStop = { x: number; color_hex: string }

export type SceneNode = SceneSpriteNode | SceneEffectNode | SceneEmitterNode

export type NativeSceneDocument = {
  schema: string
  version: number
  width: number
  height: number
  images: SceneImageSource[]
  nodes: SceneNode[]
}

export type CreateSceneImageSourceRequest = {
  key: string
  data_url: string
  filename: string
}

export type CreateSceneAssetRequest = {
  name: string
  existing_asset_id?: string | null
  base_asset_id?: string | null
  base_image_data_url?: string | null
  base_image_filename?: string | null
  extra_images: CreateSceneImageSourceRequest[]
  nodes: SceneNode[]
}

export type EditableSceneImage = {
  key: string
  data_url: string
  filename: string
}

export type EditableSceneAsset = {
  asset: AssetMetadata
  document: NativeSceneDocument
  images: EditableSceneImage[]
}

export type MonitorInfo = {
  id: string
  output_name: string
  description: string
  make: string
  model: string
  serial: string | null
  width: number
  height: number
  x: number
  y: number
  scale: number
  refresh_rate: number
  focused: boolean
  disabled: boolean
}

export type PausePolicy = {
  pause_on_fullscreen: boolean
  pause_on_battery: boolean
  fps_limit: number
}

export type RendererBackend = 'image' | 'shader' | 'video' | 'scene' | 'web'

export type RendererLaunchSpec = {
  monitor_id: string
  output_name: string
  asset: AssetMetadata
  backend: RendererBackend
}

export type RendererSessionStatus =
  | {
      kind: 'ready'
      output_name: string
      configured: boolean
      persistent: boolean
      paused_reason?: string | null
      detail?: string | null
    }
  | {
      kind: 'unsupported'
      reason: string
    }
  | {
      kind: 'failed'
      reason: string
    }

export type RendererSession = {
  spec: RendererLaunchSpec
  status: RendererSessionStatus
}

export type RuntimePlan = {
  sessions: RendererSession[]
  unresolved_assignments: string[]
}

export type RuntimeEvent = {
  runtime_key: string
  level: string
  message: string
}

export type RendererDependencyStatus = {
  available: boolean
  mode?: string | null
  detail?: string | null
}

export type RuntimeDependencies = {
  video: RendererDependencyStatus
}

export type FeatureFlags = {
  workshop_enabled: boolean
}

export type DaemonState = {
  monitors: MonitorInfo[]
  assignments: MonitorAssignment[]
  pause: PausePolicy
  runtime_dependencies: RuntimeDependencies
  runtime: RuntimePlan
  recent_events: RuntimeEvent[]
}

export type RuntimeSnapshot = {
  state: DaemonState
  assets: AssetMetadata[]
  featureFlags: FeatureFlags
  source: 'tauri' | 'tauri_disconnected' | 'mock'
  error?: string
}
