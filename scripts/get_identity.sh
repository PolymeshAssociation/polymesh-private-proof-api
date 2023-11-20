#!/bin/sh
#
POLYMESH_REST_URL="http://comp002:3001"

curl -s \
  "${POLYMESH_REST_URL}/identities/$1" \
  -H 'accept: application/json' | json_pp

