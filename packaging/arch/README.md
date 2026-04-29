# Arch / CachyOS Packaging

This directory contains the first native Arch-style package layout for Backlayer.

## What the package installs

- `backlayer-ui` in `/usr/bin`
- `backlayerd` in `/usr/bin`
- renderer workers in `/usr/bin`
- built-in demo assets in `/usr/share/backlayer/assets`
- the Plasma wallpaper plugin in `/usr/share/plasma/wallpapers/dev.madsens.backlayer.wallpaper`
- a desktop entry in `/usr/share/applications/backlayer.desktop`
- a user service in `/usr/lib/systemd/user/backlayerd.service`

## Daemon behavior

The package supports two daemon startup paths:

1. Launching the app:
   - `backlayer-ui` attempts to connect to `~/.config/backlayer/backlayer.sock`
   - if the socket is missing, it spawns `backlayerd --serve`
2. User service:
   - enable `backlayerd.service` with `systemctl --user enable --now backlayerd.service`
   - this is the preferred path for autostart on login and daemon restart after crashes

The app cold-start path is mainly for convenience. The user service is the correct long-running setup.

## Build

From the repo root:

```bash
cd packaging/arch
makepkg -si -f -p PKGBUILD
```

## Notes

- Plasma users still need to select the `Backlayer` wallpaper type in Wallpaper Settings.
- Hyprland users still use the existing daemon/runtime path.
