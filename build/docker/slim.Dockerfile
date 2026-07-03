# syntax=docker/dockerfile:1
ARG RUST_VERSION=1.95

FROM docker.io/library/rust:${RUST_VERSION}-slim-bookworm AS build

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

FROM docker.io/library/rust:${RUST_VERSION}-slim-bookworm AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    build-essential \
    pkg-config \
    cmake \
    libssl-dev \
    libz-dev \
    libffi-dev \
    git \
    python3-dev \
    curl \
    xz-utils \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -u 1000 -G sudo -U -m -s /bin/bash agentc \
    && echo "agentc ALL=(ALL) NOPASSWD: /bin/chown" >> /etc/sudoers

COPY --from=build --chown=agentc:agentc /workspace/target/release/agentc /usr/local/bin/agentc
COPY --from=build --chown=agentc:agentc /workspace/LICENSE /home/agentc/LICENSE

USER agentc
ENTRYPOINT ["agentc"]
CMD ["help"]
