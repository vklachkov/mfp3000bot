FROM debian:11-slim

# Install Rust
RUN apt-get update && apt-get install -y git curl
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install target for ARMv6
RUN rustup target add arm-unknown-linux-gnueabi

# Install tools
RUN apt-get update && apt-get install -y \
    build-essential libclang-dev

# Install dependencies for ARMv6
RUN dpkg --add-architecture armel
RUN apt-get update && apt-get install -y \
    crossbuild-essential-armel libcups2-dev:armel libsane-dev:armel libssl-dev:armel

# Build
RUN mkdir -p /src
WORKDIR /src
CMD cargo build --release --target=arm-unknown-linux-gnueabi