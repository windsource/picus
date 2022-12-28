# See here for image contents: https://github.com/microsoft/vscode-dev-containers/tree/v0.245.0/containers/rust/.devcontainer/base.Dockerfile

# [Choice] Debian OS version (use bullseye on local arm64/Apple Silicon): buster, bullseye
ARG VARIANT="buster"
FROM mcr.microsoft.com/vscode/devcontainers/rust:0-${VARIANT}

RUN dpkg --add-architecture arm64 && \
    apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
    tmux \
    vim \
    gcc-aarch64-linux-gnu \
    libssl-dev:arm64 \
    libc6-dev:arm64

RUN rustup target add aarch64-unknown-linux-gnu