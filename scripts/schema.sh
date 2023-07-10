#! /bin/bash
set -e

cd ../contracts
cd art-dealer; cargo schema
cd ../cw721-base; cargo schema
cd ../cw721-metadata-onchain; cargo schema
cd ../dao-members; cargo schema
cd ../dao-multisig; cargo schema
cd ../governance; cargo schema
cd ../identityservice; cargo schema

