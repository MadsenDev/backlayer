# systemd User Service

Backlayer should be supervised by `systemd --user` so daemon-level crashes are restarted automatically.

The repo includes a service file at:

`packaging/systemd/backlayerd.service`

For Arch-style packaged installs, the package installs:

`/usr/lib/systemd/user/backlayerd.service`

## Expected install path

The unit assumes the daemon binary is installed at:

`~/.local/bin/backlayerd`

If you install it somewhere else, edit `ExecStart` before enabling the service.

For the Arch package, the installed unit already points to:

`/usr/bin/backlayerd`

## Install

```bash
mkdir -p ~/.config/systemd/user
cp packaging/systemd/backlayerd.service ~/.config/systemd/user/backlayerd.service
systemctl --user daemon-reload
systemctl --user enable --now backlayerd.service
```

## Verify

```bash
systemctl --user status backlayerd.service
journalctl --user -u backlayerd.service -f
```

## Expected behavior

- Renderer/session failures should be handled inside `backlayerd` when possible.
- If `backlayerd` itself crashes, `systemd --user` should restart it automatically.
- The manager UI should reconnect on its own once the socket comes back.
- If the service is not enabled, the packaged Tauri app can still spawn `backlayerd --serve` on demand when it starts and the socket is missing.
