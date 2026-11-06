#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
# Additional terms under AGPLv3 section 7: see LICENSE_ADDITIONAL_TERMS.md in the repository root.
#
# Builds the arca-dev image (if missing) and starts the persistent container `arca-dev` with the
# repository mounted at /work. Cargo's registry and the build directory stay inside the container
# (named volumes), so nothing heavy is written through the host mount.
#
#   tools/docker/run-arca-dev.sh [--rebuild]
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
if command -v cygpath >/dev/null 2>&1; then ROOT="$(cygpath -m "$ROOT")"; fi

if [ "${1:-}" = "--rebuild" ] || ! docker image inspect arca-dev >/dev/null 2>&1; then
    docker build -t arca-dev -f "$ROOT/tools/docker/Dockerfile.arca-dev" "$ROOT/tools/docker"
fi

if docker ps -a --format '{{.Names}}' | grep -qx arca-dev; then docker rm -f arca-dev >/dev/null; fi
MSYS_NO_PATHCONV=1 docker run -d --name arca-dev \
    -v "$ROOT:/work" \
    -v arca-dev-cargo:/root/.cargo/registry \
    -v arca-dev-target:/tmp/arca-target \
    arca-dev
docker exec arca-dev bash -c 'git config --global --add safe.directory /work && rustc --version && cargo --version && javac -version && ls "$ANDROID_HOME" "$ARCA_NDK" >/dev/null && echo "arca-dev: toolchain OK"'
