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
	  \"ticker\": \"$1\"
	}"
}

create_account() {
	curl_post "/accounts" "{}"
}

init_account_asset() {
	curl_post "/accounts/$1/assets" "{ \"ticker\": \"$2\" }"
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

init_account_asset 1 "T1"
init_account_asset 1 "T2"
init_account_asset 1 "T3"
init_account_asset 1 "T4"
init_account_asset 1 "T5"

init_account_asset 2 "T1"
init_account_asset 2 "T2"
init_account_asset 2 "T3"
init_account_asset 2 "T4"
init_account_asset 2 "T5"

