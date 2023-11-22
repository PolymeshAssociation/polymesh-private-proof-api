#!/bin/sh
#
POLYMESH_PROOF_REST_URL="${POLYMESH_PROOF_REST_URL:-http://localhost:8001/api/v1}"
SIGNER="$1"
ACCOUNT="$2"
TICKER="$3"

curl -s -X 'POST' \
  "${POLYMESH_PROOF_REST_URL}/tx/accounts/$ACCOUNT/assets/$TICKER/apply_incoming" \
  -H 'accept: application/json' \
  -H 'Content-Type: application/json' \
  -d "{
  \"signer\": \"$SIGNER\",
	\"finalize\": false
}" | json_pp

