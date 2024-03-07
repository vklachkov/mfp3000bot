FROM debian:11-slim

# Install Rust
RUN apt-get update && apt-get install -y git curl
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install target for ARMv6
RUN rustup target add arm-unknown-linux-gnueabi

# Install tools
RUN apt-get update && apt-get install -y \
    pkg-config libclang-dev crossbuild-essential-armel

# Install dependencies for ARMv6
RUN dpkg --add-architecture armel
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl-dev:armel libcups2-dev:armel libsane-dev:armel

# Setup env vars for cross compilation
ENV PKG_CONFIG_PATH="/usr/lib/arm-linux-gnueabi/pkgconfig"
ENV PKG_CONFIG_ALLOW_CROSS=1

ENV BINDGEN_EXTRA_CLANG_ARGS_arm_unknown_linux_gnueabihf="-I/usr/include"

# Build
RUN mkdir -p /src
WORKDIR /src
CMD cargo build --release --target=arm-unknown-linux-gnueabi