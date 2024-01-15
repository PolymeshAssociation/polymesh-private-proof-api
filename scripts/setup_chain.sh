#!/bin/bash

ALICE_KEY=`./get_signer.sh Alice | grep public_key | sed -e 's/.*"public_key" : "//g' -e 's/"//'`
ISSUER_KEY=`./get_signer.sh issuer1 | grep public_key | sed -e 's/.*"public_key" : "//g' -e 's/"//'`
INVESTOR_KEY=`./get_signer.sh investor1 | grep public_key | sed -e 's/.*"public_key" : "//g' -e 's/"//'`
MEDIATOR_KEY=`./get_signer.sh mediator1 | grep public_key | sed -e 's/.*"public_key" : "//g' -e 's/"//'`
# Alice
ALICE_DID=`./create-test-admins.sh Alice 2>/dev/null`
# investor1
INVESTOR_DID=`./create-test-accounts.sh investor1 2>/dev/null`
# mediator1
MEDIATOR_DID=`./create-test-accounts.sh mediator1 2>/dev/null`
# issuer1
ISSUER_DID=`./create-test-accounts.sh issuer1 2>/dev/null`

echo "Issuer: $ISSUER_KEY $ISSUER_DID"
echo "Investor: $INVESTOR_KEY $INVESTOR_DID"
echo "Mediator: $MEDIATOR_KEY $MEDIATOR_DID"
ISSUER=`./create_account.sh | grep public_key | sed -e 's/.*"public_key" : "//g' -e 's/",//'`
echo "ISSUER=${ISSUER}"
MEDIATOR=`./create_account.sh | grep public_key | sed -e 's/.*"public_key" : "//g' -e 's/",//'`
echo "MEDIATOR=${MEDIATOR}"
INVESTOR=`./create_account.sh | grep public_key | sed -e 's/.*"public_key" : "//g' -e 's/",//'`
echo "INVESTOR=${INVESTOR}"
AUDITOR=`./create_account.sh | grep public_key | sed -e 's/.*"public_key" : "//g' -e 's/",//'`
echo "AUDITOR=${AUDITOR}"

# register accounts.
./init_account.sh issuer1 $ISSUER
./init_account.sh investor1 $INVESTOR

# Create some assets.
ASSET1=`./create_asset.sh issuer1 "$AUDITOR,$MEDIATOR" $MEDIATOR_DID | grep ConfidentialAssetCreated | sed -e 's/.*ConfidentialAssetCreated" : "//' -e 's/"//'`
echo "ASSET1 = ${ASSET1}"
ASSET2=`./create_asset.sh issuer1 "$AUDITOR,$MEDIATOR" $MEDIATOR_DID | grep ConfidentialAssetCreated | sed -e 's/.*ConfidentialAssetCreated" : "//' -e 's/"//'`
echo "ASSET2 = ${ASSET2}"
ASSET3=`./create_asset.sh issuer1 "$AUDITOR,$MEDIATOR" $MEDIATOR_DID | grep ConfidentialAssetCreated | sed -e 's/.*ConfidentialAssetCreated" : "//' -e 's/"//'`
echo "ASSET3 = ${ASSET3}"
ASSET4=`./create_asset.sh issuer1 "$AUDITOR,$MEDIATOR" $MEDIATOR_DID | grep ConfidentialAssetCreated | sed -e 's/.*ConfidentialAssetCreated" : "//' -e 's/"//'`
echo "ASSET4 = ${ASSET4}"

# mint
./asset_mint.sh issuer1 $ISSUER $ASSET1 1000000
./asset_mint.sh issuer1 $ISSUER $ASSET2 1000000
./asset_mint.sh issuer1 $ISSUER $ASSET3 1000000
./asset_mint.sh issuer1 $ISSUER $ASSET4 1000000

