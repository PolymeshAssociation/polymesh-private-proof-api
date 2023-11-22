#!/bin/sh
#
POLYMESH_PROOF_REST_URL="${POLYMESH_PROOF_REST_URL:-http://localhost:8001/api/v1}"
SIGNER="$1"
TX_ID="$2"
LEGS="$3"

curl -s -X 'POST' \
  "${POLYMESH_PROOF_REST_URL}/tx/settlements/$TX_ID/execute" \
  -H 'accept: application/json' \
  -H 'Content-Type: application/json' \
  -d "{
  \"signer\": \"$SIGNER\",
	\"finalize\": false,
  \"leg_count\": $LEGS
}" | json_pp

