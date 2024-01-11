#!/bin/bash

ALICE_KEY=`./get_signer.sh Alice | grep public_key | sed -e 's/.*"public_key" : "//g' -e 's/"//'`
ISSUER_KEY=`./get_signer.sh issuer1 | grep public_key | sed -e 's/.*"public_key" : "//g' -e 's/"//'`
INVESTOR_KEY=`./get_signer.sh investor1 | grep public_key | sed -e 's/.*"public_key" : "//g' -e 's/"//'`
MEDIATOR_KEY=`./get_signer.sh mediator1 | grep public_key | sed -e 's/.*"public_key" : "//g' -e 's/"//'`
# Alice
DID=`./get_signer_did.sh Alice`
if [[ "$DID" == "null" ]]; then
	echo "Create admin: Alice"
	./create-test-admins.sh $ALICE_KEY
fi
# investor1
INVESTOR_DID=`./get_signer_did.sh investor1 | sed -e 's/"//g'`
if [[ "$INVESTOR_DID" == "null" ]]; then
	echo "Onboard: investor1"
	./create-test-accounts.sh $INVESTOR_KEY
	INVESTOR_DID=`./get_signer_did.sh $INVESTOR_KEY | sed -e 's/"//g'`
fi
# mediator1
MEDIATOR_DID=`./get_signer_did.sh mediator1 | sed -e 's/"//g'`
if [[ "$MEDIATOR_DID" == "null" ]]; then
	echo "Onboard: mediator1"
	./create-test-accounts.sh $MEDIATOR_KEY
	MEDIATOR_DID=`./get_signer_did.sh $MEDIATOR_KEY | sed -e 's/"//g'`
fi
# issuer1
ISSUER_DID=`./get_signer_did.sh issuer1 | sed -e 's/"//g'`
if [[ "$ISSUER_DID" == "null" ]]; then
	echo "Onboard: issuer1"
	./create-test-accounts.sh $ISSUER_KEY
	ISSUER_DID=`./get_signer_did.sh $ISSUER_KEY | sed -e 's/"//g'`
fi

echo "Issuer: $ISSUER_KEY $ISSUER_DID"
echo "Investor: $INVESTOR_KEY $INVESTOR_DID"
echo "Mediator: $MEDIATOR_KEY $MEDIATOR_DID"
ISSUER=`./create_account.sh | grep public_key | sed -e 's/.*"public_key" : "//g' -e 's/",//'`
echo "ISSUER=${ISSUER}"
MEDIATOR=`./create_account.sh | grep public_key | sed -e 's/.*"public_key" : "//g' -e 's/",//'`
echo "MEDIATOR=${MEDIATOR}"
INVESTOR=`./create_account.sh | grep public_key | sed -e 's/.*"public_key" : "//g' -e 's/",//'`
echo "INVESTOR=${INVESTOR}"

# register accounts.
./init_account.sh issuer1 $ISSUER
./init_account.sh investor1 $INVESTOR

# Create some assets.
ASSET1=`./create_asset.sh issuer1 $MEDIATOR $MEDIATOR_DID | grep ConfidentialAssetCreated | sed -e 's/.*ConfidentialAssetCreated" : "//' -e 's/"//'`
echo "ASSET1 = ${ASSET1}"
ASSET2=`./create_asset.sh issuer1 $MEDIATOR $MEDIATOR_DID | grep ConfidentialAssetCreated | sed -e 's/.*ConfidentialAssetCreated" : "//' -e 's/"//'`
echo "ASSET2 = ${ASSET2}"
ASSET3=`./create_asset.sh issuer1 $MEDIATOR $MEDIATOR_DID | grep ConfidentialAssetCreated | sed -e 's/.*ConfidentialAssetCreated" : "//' -e 's/"//'`
echo "ASSET3 = ${ASSET3}"
ASSET4=`./create_asset.sh issuer1 $MEDIATOR $MEDIATOR_DID | grep ConfidentialAssetCreated | sed -e 's/.*ConfidentialAssetCreated" : "//' -e 's/"//'`
echo "ASSET4 = ${ASSET4}"

# mint
./asset_mint.sh issuer1 $ISSUER $ASSET1 1000000
./asset_mint.sh issuer1 $ISSUER $ASSET2 1000000
./asset_mint.sh issuer1 $ISSUER $ASSET3 1000000
./asset_mint.sh issuer1 $ISSUER $ASSET4 1000000

