FROM rust:latest as builder

COPY . /gerris
WORKDIR /gerris

RUN cargo build --release

FROM ubuntu:latest as container

COPY --from=builder /gerris/target/release/gerris /usr/bin/gerris

RUN apt-get update && apt-get install -y python3
