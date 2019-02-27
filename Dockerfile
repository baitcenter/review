FROM clux/muslrust:1.32.0-stable as builder

WORKDIR /work/

COPY ./api_service ./api_service
COPY ./cli ./cli
COPY ./db ./db
COPY ./migrations ./migrations
COPY ./src/ ./src/
COPY ./.env ./.env
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./diesel.toml ./diesel.toml

RUN cargo install diesel_cli -f --no-default-features --features "sqlite"
RUN diesel setup
RUN cargo build --release

FROM scratch

COPY --from=builder /work/target/x86_64-unknown-linux-musl/release/review .
COPY --from=builder /work/api_service/config/reviewd_config.json .
COPY --from=builder /work/.env .
COPY --from=builder /work/central_repo.db .
#EXPOSE 8080

ENTRYPOINT [ "./review", "reviewd", "--config", "reviewd_config.json" ]
