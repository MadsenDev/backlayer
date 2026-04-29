#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
VERSION="${1:-$(sed -n 's/^version = "\(.*\)"/\1/p' "${REPO_ROOT}/Cargo.toml" | head -n1)}"
SOURCE_SHA="${2:-}"
TEMPLATE="${SCRIPT_DIR}/backlayer/PKGBUILD.in"
PKG_DIR="${SCRIPT_DIR}/backlayer"
PKGBUILD_PATH="${PKG_DIR}/PKGBUILD"
SRCINFO_PATH="${PKG_DIR}/.SRCINFO"
SOURCE_ARCHIVE="${REPO_ROOT}/dist/release/backlayer-${VERSION}-source.tar.gz"

if [[ -z "${SOURCE_SHA}" ]]; then
  if [[ ! -f "${SOURCE_ARCHIVE}" ]]; then
    echo "missing source archive: ${SOURCE_ARCHIVE}" >&2
    echo "run ./packaging/release/create-source-tarball.sh first or pass an explicit sha256" >&2
    exit 1
  fi
  SOURCE_SHA="$(sha256sum "${SOURCE_ARCHIVE}" | awk '{print $1}')"
fi

sed \
  -e "s/@PKGVER@/${VERSION}/g" \
  -e "s/@SOURCE_SHA256@/${SOURCE_SHA}/g" \
  "${TEMPLATE}" > "${PKGBUILD_PATH}"

(
  cd "${PKG_DIR}"
  makepkg --printsrcinfo > "${SRCINFO_PATH}"
)
