ARG VARIANT=1.95-bookworm
FROM rust:${VARIANT}

ARG TARGETARCH=amd64
ARG NODE_VERSION=25.5.0

RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    # Remove imagemagick due to https://security-tracker.debian.org/tracker/CVE-2019-10131
    && apt-get purge -y imagemagick imagemagick-6-common \
    && apt-get install -y sudo jq bat curl git cmake zsh locales libssl-dev libz-dev \
    && echo "en_US.UTF-8 UTF-8" > /etc/locale.gen \
    && locale-gen \
    && update-locale LANG=en_US.UTF-8 LC_ALL=en_US.UTF-8 \
    && rm -rf /var/lib/apt/lists/*

ENV LANG=en_US.UTF-8
ENV LANGUAGE=en_US:en
ENV LC_ALL=en_US.UTF-8

RUN useradd -u 1000 -G sudo -U -m -s /usr/bin/zsh dev \
    && echo "dev ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers \
    && mkdir -p /home/dev && chown -R dev:dev /home/dev

ENV HOME=/home/dev
ENV PATH="/home/dev/.local/bin:${PATH}"

ENV CARGO_HOME=/home/dev/.cargo
RUN mkdir -p /home/dev/.cargo && chown -R dev:dev /home/dev/.cargo

COPY --from=ghcr.io/j178/prek:v0.3.2 /prek /usr/local/bin/prek
COPY --from=docker.io/jnorwood/helm-docs:v1.14.2 /usr/bin/helm-docs /usr/local/bin/helm-docs
RUN wget https://github.com/mikefarah/yq/releases/latest/download/yq_linux_amd64 -O /usr/local/bin/yq && \
    chmod +x /usr/local/bin/yq

ENV HELM_VERSION=v4.1.1
RUN curl -LO "https://get.helm.sh/helm-${HELM_VERSION}-linux-amd64.tar.gz" && \
    tar -zxvf "helm-${HELM_VERSION}-linux-amd64.tar.gz" && \
    mv linux-amd64/helm /usr/local/bin/helm && \
    rm "helm-${HELM_VERSION}-linux-amd64.tar.gz"

RUN curl -LO https://dl.k8s.io/release/v1.35.0/bin/linux/amd64/kubectl && \
    mv kubectl /usr/local/bin/kubectl

RUN sh -c "$(curl --location https://taskfile.dev/install.sh)" -- -d -b /usr/local/bin
ADD .taskfile .taskfile
COPY Taskfile.yml ./Taskfile.yml
RUN task devtools:install

RUN cargo install rust-script
ENV PATH="/home/dev/.cargo/bin:${PATH}"

RUN case "$TARGETARCH" in \
      amd64) NODE_ARCH="x64" ;; \
      arm64) NODE_ARCH="arm64" ;; \
      *) echo "Unsupported arch: $TARGETARCH" && exit 1 ;; \
    esac && \
    curl -fsSL "https://nodejs.org/dist/v$NODE_VERSION/node-v$NODE_VERSION-linux-$NODE_ARCH.tar.xz" \
    | tar -xJ --strip-components=1 -C /usr/local

RUN npm install -g npm@latest pnpm@11.1.2
RUN npm install -g playwright@${PLAYWRIGHT_VERSION}
RUN chown -R dev:dev /home/dev/.npm

USER dev

RUN curl -fsSL https://claude.ai/install.sh | bash

RUN sh -c "$(wget -O- https://github.com/deluan/zsh-in-docker/releases/download/v1.2.0/zsh-in-docker.sh)" -- \
    -t lambda
