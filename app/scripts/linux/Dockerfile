# Build environment for Sparky's Linux packages. Ubuntu 22.04's glibc keeps
# the binary runnable on Ubuntu 22.04 and newer.
FROM ubuntu:22.04
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential pkg-config curl ca-certificates file binutils \
        libsqlite3-dev libxkbcommon-dev libwayland-dev libx11-dev libgl1-mesa-dev \
    && rm -rf /var/lib/apt/lists/*
ENV RUSTUP_HOME=/opt/rustup CARGO_HOME=/opt/cargo PATH=/opt/cargo/bin:$PATH
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
WORKDIR /src
