#!/bin/sh
#
POLYMESH_PROOF_REST_URL="http://127.0.0.1:8001/api/v1"

curl -s -X 'POST' \
  "${POLYMESH_PROOF_REST_URL}/tx/assets/create_venue" \
  -H 'accept: application/json' \
  -H 'Content-Type: application/json' \
  -d "{
  \"signer\": \"$1\",
	\"finalize\": false
}" | json_pp

