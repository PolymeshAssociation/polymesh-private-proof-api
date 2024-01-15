#!/bin/sh
#
POLYMESH_PROOF_REST_URL="${POLYMESH_PROOF_REST_URL:-http://localhost:8001/api/v1}"
SIGNER="$1"
AUDITOR=`echo $2 | sed -e 's/\(0x[a-fA-F0-9]*\)/"\1"/g'`
MEDIATOR="$3"

curl -s -X 'POST' \
  "${POLYMESH_PROOF_REST_URL}/tx/assets/create_asset" \
  -H 'accept: application/json' \
  -H 'Content-Type: application/json' \
  -d "{
  \"signer\": \"$SIGNER\",
	\"finalize\": false,
  \"auditors\": [
		$AUDITOR
	],
  \"mediators\": [
		\"$MEDIATOR\"
	]
}" | json_pp

