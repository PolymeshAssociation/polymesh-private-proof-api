#!/bin/sh
#
POLYMESH_PROOF_REST_URL="${POLYMESH_PROOF_REST_URL:-http://localhost:8001/api/v1}"

curl -s \
  "${POLYMESH_PROOF_REST_URL}/signers/$1" \
  -H 'accept: application/json' | json_pp

