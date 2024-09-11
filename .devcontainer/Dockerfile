FROM ubuntu:24.04

ARG TARGETARCH
ARG USERNAME=vscode
ARG RUST_VERSION=1.80.1

# All the packages we need
RUN export DEBIAN_FRONTEND=noninteractive \
    && apt update && \
    if [ "$TARGETARCH" = "amd64" ]; then \
        apt install -y gcc-aarch64-linux-gnu; \
    elif [ "$TARGETARCH" = "arm64" ]; then \
        apt install -y gcc-x86-64-linux-gnu; \
    fi \
    && apt -y install \
    # Basics
    git \
    zsh \
    curl \
    iputils-ping \
    sudo \
    tmux \
    vim \
    # Build
    build-essential \
    musl-tools \
    # Cleanup
    && rm -rf /var/lib/apt/lists/*

# User management
RUN (userdel -r ubuntu || true) \
    && groupadd rustlang \
    && useradd -s /bin/bash -d /home/${USERNAME} -m -G rustlang ${USERNAME} \
    && echo "${USERNAME} ALL=(root) NOPASSWD:ALL" > /etc/sudoers.d/${USERNAME}

# Prepare shells
USER ${USERNAME}
COPY --chown=${USERNAME}:${USERNAME} dot_bashrc /home/${USERNAME}/.bashrc
COPY --chown=${USERNAME}:${USERNAME} dot_zshrc /home/${USERNAME}/.zshrc
COPY --chown=${USERNAME}:${USERNAME} dot_tmux.conf /home/${USERNAME}/.tmux.conf
RUN curl -sS https://starship.rs/install.sh | sh -s -- -y \
    && echo 'eval "$(starship init bash)"' >> /home/${USERNAME}/.bashrc \
    && echo 'eval "$(starship init zsh)"' >> /home/${USERNAME}/.zshrc
COPY --chown=${USERNAME}:${USERNAME} starship.toml /home/${USERNAME}/.config/
USER root

# Rust
ENV RUSTUP_HOME=/usr/local/rustup
ENV CARGO_HOME=/usr/local/cargo
ENV PATH=/usr/local/cargo/bin:$PATH
RUN mkdir -p -m 2777 $RUSTUP_HOME \
    && chgrp rustlang $RUSTUP_HOME \
    && mkdir -p -m 2777 $CARGO_HOME \
    && chgrp rustlang $CARGO_HOME \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
       sh -s -- -y --default-toolchain ${RUST_VERSION} --target x86_64-unknown-linux-musl aarch64-unknown-linux-musl --no-modify-path \
    && chmod -R ag+w $RUSTUP_HOME $CARGO_HOME

# Install just
RUN if [ "$TARGETARCH" = "amd64" ]; then \
        ITEMARCH="x86_64"; \
    elif [ "$TARGETARCH" = "arm64" ]; then \
        ITEMARCH="aarch64"; \
    else \
        exit 1; \
    fi; \
    curl -sL https://github.com/casey/just/releases/download/1.35.0/just-1.35.0-${ITEMARCH}-unknown-linux-musl.tar.gz | \
    tar xz --directory=/usr/local/bin just \
    && echo 'source <(just --completions bash)' >> /home/${USERNAME}/.bashrc \
    && echo 'source <(just --completions zsh)' >> /home/${USERNAME}/.zshrc
