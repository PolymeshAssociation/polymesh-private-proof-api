#!/bin/bash
#

URL="http://127.0.0.1:8001/api/v1"

curl_post() {
	curl -s -S --request POST \
	  --url "$URL$1" \
	  --header 'content-type: application/json' \
	  --data "$2" | json_pp
}

create_user() {
	curl_post "/users" "{
	  \"username\": \"$1\"
	}"
}

create_asset() {
	curl_post "/assets" "{
	  \"asset_id\": \"$1\"
	}"
}

create_account() {
	curl_post "/accounts" "{}"
}

init_account_asset() {
	curl_post "/accounts/$1/assets" "{ \"asset_id\": $2 }"
}

create_user "Test1"
create_user "Test2"
create_user "Test3"
create_user "Test4"
create_user "Test5"

create_asset "T1"
create_asset "T2"
create_asset "T3"
create_asset "T4"
create_asset "T5"
create_asset "T6"

create_account
create_account
create_account
create_account
create_account

init_account_asset 1 1
init_account_asset 1 2
init_account_asset 1 3
init_account_asset 1 4
init_account_asset 1 5

init_account_asset 2 1
init_account_asset 2 2
init_account_asset 2 3
init_account_asset 2 4
init_account_asset 2 5

