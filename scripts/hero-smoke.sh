#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "Building tamtri-core (mock-acp-agent + fixtures)…"
cargo build -p tamtri-core

echo "Running Milestone 5 artifact integration smoke…"
cargo test -p tamtri-core --test milestone5_artifact_integration -- --nocapture

echo "Running Milestone 8 phase tests…"
cargo test -p tamtri-core milestone8 -- --test-threads=1 --nocapture
cargo test -p tamtri-core --lib diagnostics -- --test-threads=1 --nocapture

echo "Hero smoke passed."
