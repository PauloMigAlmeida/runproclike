FROM rust:1-bullseye AS builder
WORKDIR /src
ADD . /src
RUN cargo build --release

FROM scratch AS output
ARG TARGETOS
ARG TARGETARCH
COPY --from=builder src/target/release/runproclike runproclike-${TARGETOS}-${TARGETARCH}