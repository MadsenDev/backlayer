#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ID="dev.madsens.backlayer.wallpaper"
SOURCE_DIR="${SCRIPT_DIR}/backlayer-wallpaper"
TARGET_DIR="${HOME}/.local/share/plasma/wallpapers/${PLUGIN_ID}"

if [[ ! -f "${SOURCE_DIR}/metadata.json" ]]; then
  echo "Backlayer KDE plugin metadata not found at ${SOURCE_DIR}/metadata.json" >&2
  exit 1
fi

mkdir -p "$(dirname "${TARGET_DIR}")"
rm -rf "${TARGET_DIR}"
cp -a "${SOURCE_DIR}" "${TARGET_DIR}"

echo "Installed Backlayer Plasma wallpaper plugin to: ${TARGET_DIR}"
echo
echo "Next steps:"
echo "  1) Open System Settings -> Wallpaper"
echo "  2) Select wallpaper type: Backlayer"
echo
read -r -p "Restart plasmashell now? [y/N] " RESTART_REPLY

if [[ "${RESTART_REPLY}" =~ ^[Yy]$ ]]; then
  if command -v kquitapp6 >/dev/null 2>&1 && command -v kstart >/dev/null 2>&1; then
    kquitapp6 plasmashell || true
    kstart plasmashell
    echo "Restarted plasmashell via kquitapp6/kstart"
  elif command -v systemctl >/dev/null 2>&1 && systemctl --user --quiet status plasma-plasmashell.service >/dev/null 2>&1; then
    systemctl --user restart plasma-plasmashell.service
    echo "Restarted plasma-plasmashell.service"
  else
    echo "Could not restart plasmashell automatically. Please log out/in or restart manually."
  fi
else
  echo "Skipped plasmashell restart. Restart manually if the wallpaper type is not visible yet."
fi
