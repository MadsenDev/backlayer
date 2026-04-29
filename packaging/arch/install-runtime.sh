#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <repo-root> <pkgdir>" >&2
  exit 1
fi

REPO_ROOT="$1"
PKGDIR="$2"

cd "${REPO_ROOT}"

install -Dm755 target/release/backlayerd "${PKGDIR}/usr/bin/backlayerd"
install -Dm755 target/release/backlayer-ui "${PKGDIR}/usr/bin/backlayer-ui"
install -Dm755 target/release/scene-runner "${PKGDIR}/usr/bin/scene-runner"
install -Dm755 target/release/shader-runner "${PKGDIR}/usr/bin/shader-runner"
install -Dm755 target/release/video-runner "${PKGDIR}/usr/bin/video-runner"
install -Dm755 target/release/web-runner "${PKGDIR}/usr/bin/web-runner"
ln -sf /usr/bin/backlayer-ui "${PKGDIR}/usr/bin/backlayer"

install -Dm644 packaging/systemd/backlayerd-arch.service \
  "${PKGDIR}/usr/lib/systemd/user/backlayerd.service"

install -Dm644 apps/ui/src-tauri/icons/512x512.png \
  "${PKGDIR}/usr/share/icons/hicolor/512x512/apps/backlayer.png"

install -Dm644 apps/ui/src-tauri/icons/128x128.png \
  "${PKGDIR}/usr/share/icons/hicolor/128x128/apps/backlayer.png"

install -Dm644 packaging/arch/backlayer.desktop \
  "${PKGDIR}/usr/share/applications/backlayer.desktop"

install -Dm644 integrations/kde-plasma/backlayer-wallpaper/metadata.json \
  "${PKGDIR}/usr/share/plasma/wallpapers/dev.madsens.backlayer.wallpaper/metadata.json"
install -Dm644 integrations/kde-plasma/backlayer-wallpaper/contents/ui/main.qml \
  "${PKGDIR}/usr/share/plasma/wallpapers/dev.madsens.backlayer.wallpaper/contents/ui/main.qml"

install -dm755 "${PKGDIR}/usr/share/backlayer/assets"
cp -a assets/. "${PKGDIR}/usr/share/backlayer/assets/"
