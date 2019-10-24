FROM ubuntu:19.10 as builder

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    RUST_VERSION=1.38.0

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
    rustArch='x86_64-unknown-linux-gnu'; \
    url="https://static.rust-lang.org/rustup/archive/1.20.2/${rustArch}/rustup-init"; \
    wget "$url"; \
    chmod +x rustup-init; \
    ./rustup-init -y --no-modify-path --default-toolchain $RUST_VERSION; \
    rm rustup-init; \
    chmod -R a+w $RUSTUP_HOME $CARGO_HOME; \
    apt-get remove -y --auto-remove \
    wget \
    ; \
    rm -rf /var/lib/apt/lists/*;

WORKDIR /work/

COPY ./reviewd ./reviewd
COPY ./client ./client
COPY ./migrations ./migrations
COPY ./src/ ./src/
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./diesel.toml ./diesel.toml

RUN cargo build --release

FROM ubuntu:19.10

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
    libpq5 \
    libssl1.1 \
    ; \
    rm -rf /var/lib/apt/lists/*;

COPY --from=builder /work/target/release/review .
EXPOSE 8080

ENTRYPOINT ["./review"]
