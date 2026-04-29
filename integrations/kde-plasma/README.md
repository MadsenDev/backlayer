# Backlayer KDE Plasma 6 Wallpaper Plugin

This integration adds a Plasma 6 wallpaper plugin package that appears in the wallpaper type selector as **Backlayer**.

## Current behavior

The plugin is already usable on Plasma 6 and can render Backlayer wallpapers directly inside Plasma wallpaper context.

- Plugin installs to the Plasma wallpapers package path.
- Plugin appears as a wallpaper type named **Backlayer**.
- Plugin reads `~/.config/backlayer/config.toml` on a timer and applies the current assigned wallpaper.
- Plugin renders:
  - `image` wallpapers
  - `video` wallpapers through `QtMultimedia`
  - a simplified native `scene` path with sprite, glow, and particle support in QML
- Plugin currently does not render:
  - `shader` wallpapers
  - `web` wallpapers

This is still a Plasma-specific adapter, not a full reuse of the Hyprland layer-shell runtime.

## Layout

```text
integrations/kde-plasma/
  backlayer-wallpaper/
    metadata.json
    contents/
      ui/
        main.qml
  install.sh
  README.md
```

## Install

From repository root:

```bash
./integrations/kde-plasma/install.sh
```

The script copies the package to:

```text
~/.local/share/plasma/wallpapers/dev.madsens.backlayer.wallpaper
```

It then offers an optional plasmashell restart.

If **Backlayer** does not appear immediately, restart Plasma Shell or log out/in and retry.

## Manual verification

1. Open **System Settings -> Wallpaper**.
2. Open wallpaper type selector.
3. Confirm **Backlayer** appears.
4. Select **Backlayer** and apply.
5. Assign a Backlayer wallpaper through the normal Backlayer config/UI flow.
6. Confirm the selected wallpaper renders on the Plasma desktop.

## Plasma 6 packaging notes

- Uses JSON metadata, not legacy `.desktop` plugin metadata.
- Uses `KPackageStructure: Plasma/Wallpaper`.
- Uses `X-Plasma-API-Minimum-Version: 6.0`.
- Uses root `contents/ui/main.qml` wallpaper script entry.

## Current implementation notes

- The plugin polls `~/.config/backlayer/config.toml` every 5 seconds.
- It parses the assigned wallpaper kind and entrypoint from the config file.
- `scene` wallpapers are rendered through a simplified QML scene path, not the Rust `scene-runner`.
- The current plugin path does not yet use daemon IPC, shared-memory frame transport, or Plasma-aware monitor assignment.

## Current limitations

- The config read path is file-based and polling-based.
- Per-monitor mapping is not Plasma-native yet.
- `scene` support is partial compared with the main native runtime.
- `shader` and `web` wallpapers still fall back to an unsupported status message in the plugin.
- The plugin currently assumes a readable local Backlayer config under `~/.config/backlayer/config.toml`.

## Next milestones

1. **Daemon status bridge:** add a thin IPC read helper (`backlayerctl ... --json`) and display daemon/assignment status.
2. **Monitor mapping:** resolve containment/screen identity to Backlayer monitor IDs.
3. **Live rendering prototype:** add a temporary frame source bridge (prototype) while designing efficient transport.
4. **Final rendering direction:** move to efficient shared-memory / Qt-native rendering path.

## Live-render bridge options (investigation summary)

### Option A: frame/image stream in plugin (preferred first prototype)

- Plugin displays daemon-provided frames or stream.
- Keeps KDE wallpaper ownership in Plasma.
- Good architectural fit for thin adapter model.
- Prototype can start with simple reload source; should evolve quickly to efficient transport.

### Option B: Qt/Quick renderer helper inside KDE path

- More direct rendering potential.
- Higher implementation complexity and likely C++ plugin surface.
- Best reserved until Option A constraints are measured.

### Option C: control-only plugin + external renderer

- Minimal changes.
- Likely repeats current KDE static/preview behavior and fails to establish true wallpaper composition path.
- Not sufficient as final bridge.

## Non-goals for this plugin path

- No rewrite of existing Hyprland layer-shell runtime.
- No compositor-default behavior changes.
- No polished config UI yet.
- No long-term PNG-per-frame design.


## Troubleshooting

- Plasma logging can be noisy; plugin logs may appear under user journal output from `plasmashell`.
- Helpful command:

```bash
journalctl --user -xe --unit plasma-plasmashell.service
```

- If the wallpaper type still does not show, restart shell or log out/in before further debugging.
