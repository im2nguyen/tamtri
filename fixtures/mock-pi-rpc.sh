#!/bin/sh
# Minimal Pi RPC mock for tamtri integration tests.
# Usage: TAMTRI_PI_COMMAND=/path/to/mock-pi-rpc.sh cargo test -p tamtri-core pi_native

extract_id() {
  printf '%s' "$1" | sed -n 's/.*"id"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1
}

while IFS= read -r line; do
  id=$(extract_id "$line")
  case "$line" in
    *'"type":"prompt"'*|*'"type": "prompt"'*)
      printf '%s\n' '{"type":"message_update","message":{"role":"assistant"},"assistantMessageEvent":{"type":"text_delta","delta":"tamtri-pi-ok"}}'
      printf '%s\n' '{"type":"agent_end"}'
      printf '%s\n' "{\"type\":\"response\",\"id\":\"${id:-req_1}\",\"command\":\"prompt\",\"success\":true,\"data\":{}}"
      ;;
    *'"type":"get_available_models"'*)
      printf '%s\n' "{\"type\":\"response\",\"id\":\"${id:-req_1}\",\"command\":\"get_available_models\",\"success\":true,\"data\":{\"models\":[{\"provider\":\"mock\",\"id\":\"fast\",\"name\":\"Mock Fast\"}]}}"
      ;;
    *'"type":"abort"'*)
      printf '%s\n' "{\"type\":\"response\",\"id\":\"${id:-req_1}\",\"command\":\"abort\",\"success\":true,\"data\":{}}"
      ;;
    *)
      ;;
  esac
done
