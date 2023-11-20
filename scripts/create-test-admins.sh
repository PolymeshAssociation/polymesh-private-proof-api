#!/bin/sh
#
POLYMESH_REST_URL="http://comp002:3001"

curl -s -X 'POST' \
  "${POLYMESH_REST_URL}/developer-testing/create-test-admins" \
  -H 'accept: application/json' \
  -H 'Content-Type: application/json' \
  -d "{
  \"accounts\": [
    {
      \"address\": \"$1\",
      \"initialPolyx\": 300000000.0
    }
  ]
}" | json_pp

