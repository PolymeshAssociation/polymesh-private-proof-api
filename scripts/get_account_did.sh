#!/bin/sh
#
POLYMESH_PROOF_REST_URL="${POLYMESH_PROOF_REST_URL:-http://localhost:8001/api/v1}"
ACCOUNT="$1"

curl -s \
  "${POLYMESH_PROOF_REST_URL}/tx/accounts/$ACCOUNT/account_did" \
  -H 'accept: application/json' | json_pp

