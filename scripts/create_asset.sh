#!/bin/sh
#
POLYMESH_PROOF_REST_URL="${POLYMESH_PROOF_REST_URL:-http://localhost:8001/api/v1}"
SIGNER="$1"
TICKER="$2"
AUDITOR="$3"
MEDIATOR="$4"

curl -s -X 'POST' \
  "${POLYMESH_PROOF_REST_URL}/tx/assets/create_asset" \
  -H 'accept: application/json' \
  -H 'Content-Type: application/json' \
  -d "{
  \"signer\": \"$SIGNER\",
	\"finalize\": false,
  \"ticker\": \"$TICKER\",
  \"auditors\": [
		\"$AUDITOR\"
	],
  \"mediators\": [
		\"$MEDIATOR\"
	]
}" | json_pp

