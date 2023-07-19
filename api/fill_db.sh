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

create_account_balance() {
	curl_post "/accounts/$1/balances/$2" "{}"
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

create_account_balance 1 1
create_account_balance 1 2
create_account_balance 1 3
create_account_balance 1 4
create_account_balance 1 5

create_account_balance 2 1
create_account_balance 2 2
create_account_balance 2 3
create_account_balance 2 4
create_account_balance 2 5

