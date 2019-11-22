FROM ubuntu:19.10 as builder

ARG RUSTUP_VERSION=1.20.2
ARG RUST_VERSION=1.39.0
ARG FRONTEND_VERSION=0.1.0

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    clang \
    libpq-dev \
    libssl-dev \
    make \
    pkgconf \
    wget \
    ; \
    rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN RUST_ARCH='x86_64-unknown-linux-gnu'; \
    url="https://static.rust-lang.org/rustup/archive/${RUSTUP_VERSION}/${RUST_ARCH}/rustup-init"; \
    wget "$url"; \
    chmod +x rustup-init; \
    ./rustup-init -y --no-modify-path --default-toolchain ${RUST_VERSION}; \
    rm rustup-init

WORKDIR /work/

RUN wget https://github.com/petabi/resolutions-frontend/releases/download/${FRONTEND_VERSION}/resolutions-frontend-${FRONTEND_VERSION}.tar.gz && \
    tar xvzf resolutions-frontend-${FRONTEND_VERSION}.tar.gz && \
    rm resolutions-frontend-${FRONTEND_VERSION}.tar.gz && \
    mv resolutions-frontend-${FRONTEND_VERSION} htdocs

COPY ./migrations ./migrations
COPY ./src/ ./src/
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./diesel.toml ./diesel.toml

RUN cargo install --path . && cargo clean

FROM ubuntu:19.10

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
    libpq5 \
    libssl1.1 \
    ; \
    rm -rf /var/lib/apt/lists/*;

COPY --from=builder /work/htdocs /var/htdocs
COPY --from=builder /usr/local/cargo/bin/review /usr/local/bin
EXPOSE 8080

ENV FRONTEND_DIR=/var/htdocs

ENTRYPOINT ["/usr/local/bin/review"]
