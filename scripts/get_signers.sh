#!/bin/sh
#
POLYMESH_PROOF_REST_URL="http://127.0.0.1:8001/api/v1"

curl -s \
  "${POLYMESH_PROOF_REST_URL}/signers" \
  -H 'accept: application/json' | json_pp

