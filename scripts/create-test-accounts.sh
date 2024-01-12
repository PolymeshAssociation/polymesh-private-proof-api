#!/bin/sh
#
POLYMESH_REST_URL="${POLYMESH_REST_URL:=http://localhost:3001}"
NAME="$1"

KEY=`./get_signer.sh $NAME | grep public_key | sed -e 's/.*"public_key" : "//g' -e 's/"//'`

DID=`./get_signer_did.sh $KEY | sed -e 's/"//g'`
if [[ "$DID" == "null" ]]; then
	curl -s -X 'POST' \
	  "${POLYMESH_REST_URL}/developer-testing/create-test-accounts" \
	  -H 'accept: application/json' \
	  -H 'Content-Type: application/json' \
	  -d "{
	  \"signer\": \"Alice-1\",
	  \"accounts\": [
	    {
	      \"address\": \"$KEY\",
	      \"initialPolyx\": 10000.0
	    }
	  ]
	}" | json_pp 1>&2
	DID=`./get_signer_did.sh $KEY | sed -e 's/"//g'`
fi

echo "$DID"
