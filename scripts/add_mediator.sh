#!/bin/sh
#
POLYMESH_PROOF_REST_URL="${POLYMESH_PROOF_REST_URL:-http://localhost:8001/api/v1}"

curl -s -X 'POST' \
  "${POLYMESH_PROOF_REST_URL}/tx/accounts/$2/add_mediator" \
  -H 'accept: application/json' \
  -H 'Content-Type: application/json' \
  -d "{
  \"signer\": \"$1\",
	\"finalize\": false
}" | json_pp

