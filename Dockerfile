FROM clux/muslrust:1.32.0-stable as builder

WORKDIR /work/

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src

RUN cargo build --release

FROM scratch

COPY --from=builder /work/target/x86_64-unknown-linux-musl/release/review .

ENTRYPOINT [ "./review" ]
