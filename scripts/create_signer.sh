#!/bin/sh
#
POLYMESH_PROOF_REST_URL="${POLYMESH_PROOF_REST_URL:-http://localhost:8001/api/v1}"

curl -s -X 'POST' \
  "${POLYMESH_PROOF_REST_URL}/signers" \
  -H 'accept: application/json' \
  -H 'Content-Type: application/json' \
  -d "{
  \"name\": \"$1\"
}" | json_pp

