#!/bin/sh
#
POLYMESH_PROOF_REST_URL="${POLYMESH_PROOF_REST_URL:-http://localhost:8001/api/v1}"

curl -s -X 'POST' \
  "${POLYMESH_PROOF_REST_URL}/accounts" \
  -H 'accept: application/json' | json_pp

