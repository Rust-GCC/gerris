FROM rust

WORKDIR .

RUN cargo install --path .

FROM ubuntu:22-04

COPY rust:/.cargo/bin/gerris/ /usr/local/bin/gerris

RUN apt-get update && apt-get install -y python3
