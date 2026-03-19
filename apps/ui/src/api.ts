import type { AssetMetadata, AssignmentSettings, CreateNativeAssetRequest, CreateSceneAssetRequest, DaemonState, EditableSceneAsset, FeatureFlags, RuntimeSnapshot } from './types'
import type { PausePolicy } from './types'

const emptyState: DaemonState = {
  monitors: [],
  assignments: [],
  pause: {
    pause_on_fullscreen: true,
    pause_on_battery: true,
    fps_limit: 30,
  },
  runtime_dependencies: {
    video: {
      available: false,
      mode: 'preview_fallback',
      detail: 'Mock mode',
    },
  },
  runtime: {
    sessions: [],
    unresolved_assignments: [],
  },
  recent_events: [],
}

const mockState: DaemonState = {
  ...emptyState,
  monitors: [
    {
      id: 'hypr:chimei-innolux-corporation:0x14c9:chimei-innolux-corporation-0x14c9',
      output_name: 'eDP-1',
      description: 'Chimei Innolux Corporation 0x14C9',
      make: 'Chimei Innolux Corporation',
      model: '0x14C9',
      serial: null,
      width: 1920,
      height: 1080,
      x: 0,
      y: 0,
      scale: 1,
      refresh_rate: 60.008,
      focused: true,
      disabled: false,
    },
  ],
}

const mockAssets: AssetMetadata[] = [
  {
    id: 'demo.neon-grid',
    name: 'Neon Grid',
    kind: 'shader',
    animated: false,
    image_fit: null,
    source_kind: 'native',
    preview_image: null,
    compatibility: {
      status: 'supported',
      warnings: [],
    },
    import_metadata: null,
    entrypoint: 'shaders/neon-grid.wgsl',
    asset_path: null,
  },
]

const defaultFeatureFlags: FeatureFlags = {
  workshop_enabled: false,
}

const previewDataUrlCache = new Map<string, Promise<string | null>>()

export async function fetchRuntimeSnapshot(
  options: { includeAssets?: boolean } = {},
): Promise<RuntimeSnapshot> {
  const { includeAssets = true } = options

  try {
    const { invoke } = await import('@tauri-apps/api/core')
    try {
      const [featureFlags, state, assets] = await Promise.all([
        invoke<FeatureFlags>('daemon_feature_flags'),
        invoke<DaemonState>('daemon_get_state'),
        includeAssets
          ? invoke<AssetMetadata[]>('daemon_list_assets')
          : Promise.resolve<AssetMetadata[]>(mockAssets),
      ])

      return {
        featureFlags,
        state,
        assets: includeAssets ? assets : [],
        source: 'tauri',
      }
    } catch (error) {
      return {
        featureFlags: defaultFeatureFlags,
        state: emptyState,
        assets: [],
        source: 'tauri_disconnected',
        error: error instanceof Error ? error.message : String(error),
      }
    }
  } catch (error) {
    return {
      featureFlags: defaultFeatureFlags,
      state: mockState,
      assets: mockAssets,
      source: 'mock',
      error: error instanceof Error ? error.message : String(error),
    }
  }
}

export async function assignWallpaper(
  monitorId: string,
  assetId: string,
): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core')
  await invoke('daemon_assign_wallpaper', {
    monitorId,
    assetId,
  })
}

export async function updatePausePolicy(pause: PausePolicy): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core')
  await invoke('daemon_update_pause_policy', {
    pause,
  })
}

export async function updateAssignmentSettings(
  monitorId: string,
  settings: AssignmentSettings,
): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core')
  await invoke('daemon_update_assignment_settings', {
    monitorId,
    settings,
  })
}

export async function importWorkshopPath(path: string): Promise<AssetMetadata[]> {
  const { invoke } = await import('@tauri-apps/api/core')
  return await invoke<AssetMetadata[]>('daemon_import_workshop_path', {
    path,
  })
}

export async function suggestWorkshopPaths(): Promise<string[]> {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    return await invoke<string[]>('daemon_suggest_workshop_paths')
  } catch {
    return []
  }
}

export async function reimportAsset(assetId: string): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core')
  await invoke('daemon_reimport_asset', {
    assetId,
  })
}

export async function removeAsset(assetId: string): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core')
  await invoke('daemon_remove_asset', {
    assetId,
  })
}

export async function createSceneAsset(request: CreateSceneAssetRequest): Promise<AssetMetadata> {
  const { invoke } = await import('@tauri-apps/api/core')
  return await invoke<AssetMetadata>('daemon_create_scene_asset', {
    request,
  })
}

export async function createNativeAsset(request: CreateNativeAssetRequest): Promise<AssetMetadata> {
  const { invoke } = await import('@tauri-apps/api/core')
  return await invoke<AssetMetadata>('daemon_create_native_asset', {
    request,
  })
}

export async function loadEditableSceneAsset(assetId: string): Promise<EditableSceneAsset> {
  const { invoke } = await import('@tauri-apps/api/core')
  return await invoke<EditableSceneAsset>('daemon_load_editable_scene_asset', {
    assetId,
  })
}

export async function loadAssetPreviewDataUrl(path: string): Promise<string | null> {
  const cached = previewDataUrlCache.get(path)
  if (cached) {
    return await cached
  }

  const request = (async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      return await invoke<string>('asset_preview_data_url', {
        path,
      })
    } catch {
      return null
    }
  })()

  previewDataUrlCache.set(path, request)
  return await request
}
