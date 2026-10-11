# syntax=docker/dockerfile:1
ARG RUST_VERSION=1.97
ARG PYTHON_VERSION=3.14.8
ARG NODE_VERSION=26.5.1
ARG PNPM_VERSION=11.17.0
ARG NPM_VERSION=12.0.2
ARG ESBUILD_VERSION=0.28.2
ARG UV_VERSION=0.12.17

FROM docker.io/library/rust:${RUST_VERSION}-slim-trixie AS build

ARG BUILD_TAG=""

RUN DEBIAN_FRONTEND=noninteractive apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    cmake \
    libssl-dev \
    libz-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

COPY Cargo.toml Cargo.lock rust-toolchain.toml README.md LICENSE ./
ADD crates/ ./crates/
ADD sbom/ ./sbom/
RUN BUILD_TAG=$BUILD_TAG cargo build -p agentc --all-features --release

FROM ghcr.io/astral-sh/uv:${UV_VERSION} AS uv

FROM docker.io/library/python:${PYTHON_VERSION}-slim-trixie AS runtime

ARG RUST_VERSION
ARG NODE_VERSION
ARG PNPM_VERSION
ARG NPM_VERSION
ARG ESBUILD_VERSION
ARG TARGETARCH

RUN apt-get update && \
    apt-get upgrade -y && \
    apt-get install --no-install-recommends -y \
        ca-certificates \
        build-essential \
        pkg-config \
        cmake \
        libssl-dev \
        libz-dev \
        libffi-dev \
        git \
        curl \
        python3-dev \
        # libatomic1 is required for nodejs
        libatomic1 \
        xz-utils \
    && rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PNPM_HOME=/usr/local/pnpm \
    pnpm_config_cache_dir=/usr/local/pnpm/cache \
    pnpm_config_state_dir=/usr/local/pnpm/state \
    npm_config_cache=/usr/local/npm \
    UV_CACHE_DIR=/usr/local/uv/cache \
    UV_PYTHON_INSTALL_DIR=/usr/local/uv/python \
    PATH=/usr/local/cargo/bin:$PATH \
    RUST_VERSION=${RUST_VERSION}

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain ${RUST_VERSION} --no-modify-path \
    && chmod -R a+w "${RUSTUP_HOME}" "${CARGO_HOME}"

RUN case "${TARGETARCH}" in \
    amd64) NODE_ARCH="x64" ;; \
    arm64) NODE_ARCH="arm64" ;; \
    *) echo "Unsupported arch: ${TARGETARCH}" && exit 1 ;; \
    esac \
    && curl -fsSL "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-${NODE_ARCH}.tar.xz" \
    | tar -xJ --strip-components=1 -C /usr/local

RUN npm install --global \
    npm@${NPM_VERSION} \
    pnpm@${PNPM_VERSION} \
    esbuild@${ESBUILD_VERSION} \
    && npm cache clean --force \
    && mkdir -p \
        "${PNPM_HOME}" \
        "${pnpm_config_cache_dir}" \
        "${pnpm_config_state_dir}" \
        "${npm_config_cache}" \
        "${UV_CACHE_DIR}" \
        "${UV_PYTHON_INSTALL_DIR}" \
    && chmod -R a+w \
        "${PNPM_HOME}" \
        "${npm_config_cache}" \
        "${UV_CACHE_DIR}" \
        "${UV_PYTHON_INSTALL_DIR}"

RUN useradd -u 1000 -G sudo -U -m -s /bin/bash agentc

COPY --from=uv /uv /uvx /usr/local/bin/
COPY --from=build --chown=agentc:agentc /workspace/target/release/agentc /usr/local/bin/agentc
COPY --from=build --chown=agentc:agentc /workspace/LICENSE /home/agentc/LICENSE

WORKDIR /workspace
ENTRYPOINT ["agentc"]
CMD ["help"]
