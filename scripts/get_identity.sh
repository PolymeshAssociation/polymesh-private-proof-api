#!/bin/sh
#
POLYMESH_REST_URL="${POLYMESH_REST_URL:=http://localhost:3001}"

curl -s \
  "${POLYMESH_REST_URL}/identities/$1" \
  -H 'accept: application/json' | json_pp

