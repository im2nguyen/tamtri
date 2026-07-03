#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "Building tamtri-core (mock-acp-agent + fixtures)…"
cargo build -p tamtri-core

echo "Running Milestone 5 artifact integration smoke…"
cargo test -p tamtri-core --test milestone5_artifact_integration -- --nocapture

echo "Hero smoke passed."
