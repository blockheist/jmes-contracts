#! /bin/bash
set -e

CUR_DIR=$(pwd)
{ # try
    cd ..
    pwd &&
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.12.8
    cd $CUR_DIR
} || { # catch
    cd $CUR_DIR
    echo "Exit with error .."
    exit 1
}
