#!/bin/bash

set -ue

docker build -f RPiZero.Dockerfile -t mfp3000bot_build .

docker run \
    -v $HOME/.cargo/registry:/root/.cargo/registry \
    -v $(pwd):/src \
    -t mfp3000bot_build