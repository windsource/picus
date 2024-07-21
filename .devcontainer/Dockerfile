# See here for image contents: https://github.com/microsoft/vscode-dev-containers/tree/v0.245.0/containers/rust/.devcontainer/base.Dockerfile

FROM mcr.microsoft.com/vscode/devcontainers/rust:1-1-bookworm

RUN apt update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
    tmux \
    vim 

RUN rustup target add x86_64-unknown-linux-musl \
    && rustup target add aarch64-unknown-linux-musl