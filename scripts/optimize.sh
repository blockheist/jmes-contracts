#! /bin/bash
set -e

CUR_DIR=$(pwd)
{ # try
    cd ..
  # detect if the architecture is amd64 or arm64 and run the appropriate optimizer
    if [ "$(uname -m)" = "x86_64" ]; then
        echo "Running optimizer for x86_64 architecture" &&
        docker run --rm -v "$(pwd)":/code \
        --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
        --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
        cosmwasm/workspace-optimizer:0.12.9
    elif [ "$(uname -m)" = "arm64" ]; then
        echo "Running optimizer for arm64 architecture" &&
        docker run --rm -v "$(pwd)":/code \
        --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
        --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
        cosmwasm/workspace-optimizer-arm64:0.12.11
    else
        echo "Unsupported architecture"
        exit 1
    fi
    cd $CUR_DIR
} || { # catch
    cd $CUR_DIR
    echo "Exit with error .."
    exit 1
}
