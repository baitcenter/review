FROM ubuntu:18.04 as builder

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    RUST_VERSION=1.35.0

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    g++ \
    libhyperscan-dev \
    librdkafka-dev \
    libsqlite3-dev \
    libssl-dev \
    make \
    pkgconf \
    wget \
    zlib1g-dev \
    ; \
    rustArch='x86_64-unknown-linux-gnu'; \
    url="https://static.rust-lang.org/rustup/archive/1.18.3/${rustArch}/rustup-init"; \
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

COPY ./api_service ./api_service
COPY ./client ./client
COPY ./db ./db
COPY ./migrations ./migrations
COPY ./remake/ ./remake/
COPY ./src/ ./src/
COPY ./.env ./.env
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./diesel.toml ./diesel.toml

RUN cargo install diesel_cli -f --no-default-features --features "sqlite"
RUN diesel setup
RUN cargo build --release

FROM ubuntu:18.04

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
    libhyperscan4 \
    libsqlite3-0 \
    libssl1.1 \
    ; \
    rm -rf /var/lib/apt/lists/*;

ENV LD_LIBRARY_PATH=/usr/pkg/lib

COPY --from=builder /work/target/release/review .
COPY --from=builder /work/.env .
COPY --from=builder /work/central_repo.db .
EXPOSE 8080

ENTRYPOINT ["./review"]
