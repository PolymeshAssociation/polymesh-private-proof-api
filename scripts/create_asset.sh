#!/bin/sh
#
POLYMESH_PROOF_REST_URL="${POLYMESH_PROOF_REST_URL:-http://localhost:8001/api/v1}"
SIGNER="$1"
AUDITORS=`echo $2 | sed -e 's/\(0x[a-fA-F0-9]*\)/"\1"/g'`
MEDIATORS=`echo $3 | sed -e 's/\(0x[a-fA-F0-9]*\)/"\1"/g'`

if [ "x$AUDITORS" != "x" ]; then
	AUDITORS=",\"auditors\": [ $AUDITORS ]"
fi

if [ "x$MEDIATORS" != "x" ]; then
	MEDIATORS=",\"mediators\": [ $MEDIATORS ]"
fi

curl -s -X 'POST' \
  "${POLYMESH_PROOF_REST_URL}/tx/assets/create_asset" \
  -H 'accept: application/json' \
  -H 'Content-Type: application/json' \
  -d "{
  \"signer\": \"$SIGNER\",
	\"finalize\": false
	$AUDITORS
	$MEDIATORS
}" | json_pp

