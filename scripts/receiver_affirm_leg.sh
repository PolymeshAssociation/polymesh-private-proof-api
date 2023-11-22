#!/bin/sh
#
POLYMESH_PROOF_REST_URL="${POLYMESH_PROOF_REST_URL:-http://localhost:8001/api/v1}"
SIGNER="$1"
ACCOUNT="$2"
TX_ID="$3"
LEG_ID="$4"
TICKER="$5"
AMOUNT="$6"

curl -s -X 'POST' \
  "${POLYMESH_PROOF_REST_URL}/tx/accounts/$ACCOUNT/assets/$TICKER/receiver_affirm_leg" \
  -H 'accept: application/json' \
  -H 'Content-Type: application/json' \
  -d "{
  \"signer\": \"$SIGNER\",
	\"finalize\": false,
  \"transaction_id\": $TX_ID,
  \"leg_id\": $LEG_ID,
  \"amount\": $AMOUNT
}" | json_pp

