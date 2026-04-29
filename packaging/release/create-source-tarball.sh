#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
VERSION="${1:-$(sed -n 's/^version = "\(.*\)"/\1/p' "${REPO_ROOT}/Cargo.toml" | head -n1)}"
GIT_REF="${2:-HEAD}"
OUT_DIR="${3:-${REPO_ROOT}/dist/release}"
ARCHIVE_NAME="backlayer-${VERSION}-source.tar.gz"
ARCHIVE_PATH="${OUT_DIR}/${ARCHIVE_NAME}"
SHA_PATH="${ARCHIVE_PATH}.sha256"

mkdir -p "${OUT_DIR}"
cd "${REPO_ROOT}"

git archive \
  --format=tar.gz \
  --prefix="backlayer-${VERSION}/" \
  -o "${ARCHIVE_PATH}" \
  "${GIT_REF}"

sha256sum "${ARCHIVE_PATH}" | tee "${SHA_PATH}"
