#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="${1:-$(pwd)}"

cd "${REPO_ROOT}"

pnpm install --frozen-lockfile
pnpm --dir apps/ui build

cargo build --release \
  -p backlayerd \
  -p backlayer-ui \
  -p scene-runner \
  -p shader-runner \
  -p video-runner \
  -p web-runner
