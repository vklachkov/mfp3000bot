#!/usr/bin/env bash

set -ue

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
IMAGE_NAME=mfp3000bot_build

docker build -f "$SCRIPT_DIR/RPiZero.Dockerfile" -t "$IMAGE_NAME" .

docker run \
    -v "$HOME/.cargo/registry:/root/.cargo/registry" \
    -v "$SCRIPT_DIR/..:/src" \
    -t "$IMAGE_NAME"