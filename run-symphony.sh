#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SYMPHONY_DIR="${SYMPHONY_DIR:-/work/symphony/elixir}"
MISE_BIN="${MISE_BIN:-/home/jordan/.local/bin/mise}"
PORT="${SYMPHONY_PORT:-8080}"
WORKFLOW_FILE="${WORKFLOW_FILE:-$ROOT_DIR/WORKFLOW.md}"
ENV_FILE="${ENV_FILE:-$ROOT_DIR/.env}"

if [[ ! -d "$SYMPHONY_DIR" ]]; then
  echo "Symphony directory not found: $SYMPHONY_DIR" >&2
  exit 1
fi

if [[ ! -x "$MISE_BIN" ]]; then
  echo "mise binary not found or not executable: $MISE_BIN" >&2
  exit 1
fi

if [[ ! -f "$WORKFLOW_FILE" ]]; then
  echo "Workflow file not found: $WORKFLOW_FILE" >&2
  exit 1
fi

if [[ -f "$ENV_FILE" ]]; then
  set -a
  # shellcheck disable=SC1090
  . "$ENV_FILE"
  set +a
fi

export LANG="${LANG:-C.UTF-8}"
export LC_ALL="${LC_ALL:-C.UTF-8}"
export ELIXIR_ERL_OPTIONS="${ELIXIR_ERL_OPTIONS:-+fnu}"

cd "$SYMPHONY_DIR"
exec "$MISE_BIN" exec -- ./bin/symphony \
  --i-understand-that-this-will-be-running-without-the-usual-guardrails \
  --port "$PORT" \
  "$WORKFLOW_FILE"
