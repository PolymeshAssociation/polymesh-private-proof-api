#!/bin/sh
#
POLYMESH_PROOF_REST_URL="${POLYMESH_PROOF_REST_URL:-http://localhost:8001/api/v1}"
SIGNER="$1"
VENUE="$2"
TICKER="$3"
SENDER="$4"
RECEIVER="$5"
MEDIATOR="$6"

curl -s -X 'POST' \
  "${POLYMESH_PROOF_REST_URL}/tx/venues/$VENUE/settlement/create" \
  -H 'accept: application/json' \
  -H 'Content-Type: application/json' \
  -d "{
  \"signer\": \"$SIGNER\",
	\"finalize\": false,
  \"legs\": [{
		\"ticker\": \"$TICKER\",
  	\"sender\": \"$SENDER\",
  	\"receiver\": \"$RECEIVER\",
  	\"mediators\": [
			\"$MEDIATOR\"
		]
	}]
}" | json_pp

