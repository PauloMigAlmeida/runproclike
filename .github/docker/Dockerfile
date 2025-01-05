FROM rust:1-bullseye AS builder
WORKDIR /src
ADD . /src
RUN cargo build --release

FROM debian:bullseye
COPY --from=builder src/target/release/runproclike /usr/bin/runproclike
ENTRYPOINT ["/usr/bin/runproclike"]