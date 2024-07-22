# This dev container is supposed to work on amd64 and arm64 platform and
# contains all tools to build for both platform

FROM mcr.microsoft.com/vscode/devcontainers/rust:1-1-bookworm

ARG BUILDARCH

RUN if [ "$BUILDARCH" = "amd64" ]; then \
        dpkg --add-architecture arm64 && \
        apt update && \
        apt install -y gcc-aarch64-linux-gnu musl-tools libc6-dev:arm64; \
    elif [ "$BUILDARCH" = "arm64" ]; then \
        apt update && \
        apt install -y gcc-x86-64-linux-gnu musl-tools; \
    fi \
    && apt install -y --no-install-recommends tmux vim 

RUN rustup target add x86_64-unknown-linux-musl \
    && rustup target add aarch64-unknown-linux-musl