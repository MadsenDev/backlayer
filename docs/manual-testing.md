# Manual Verification Matrix

Use this as the release-hardening checklist for the current MVP.

## Daemon And UI

- Start `backlayerd --serve`
- Open the Tauri manager
- Confirm the first runtime snapshot loads
- Confirm the manager reconnects if the daemon is restarted

## Wallpaper Assignment

- Assign a native image wallpaper
- Assign a shader wallpaper
- Assign a video wallpaper
- Assign a native scene wallpaper
- Confirm each appears on the selected monitor

## Runtime Policy

- Enable `pause_on_fullscreen` and confirm animated wallpapers pause
- Enable `pause_on_battery` and confirm animated wallpapers pause on battery power
- Lower `fps_limit` and confirm animation cadence visibly drops

## Monitor Behavior

- Change monitor layout while the daemon is running
- Unplug/replug an external monitor if available
- Confirm assignments stay bound to the correct output identity
- Confirm the daemon does not require a restart to reconcile monitor changes

## Input And Placement

- Confirm wallpaper surfaces stay behind normal windows
- Confirm clicks and keyboard input pass through to normal desktop/application windows

## Recovery

- Kill a shader/video/scene runner process
- Confirm the daemon reports the failure and restarts the renderer path
- Confirm the UI remains connected to the daemon

## Native Create Flows

- Create a native still image wallpaper
- Create a native video wallpaper
- Create a native shader wallpaper
- Create and edit a native scene wallpaper
- Confirm each created asset appears in the browser and can be assigned
