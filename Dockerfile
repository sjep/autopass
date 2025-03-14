FROM alpine:latest

RUN apk update && apk add build-base curl

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > /rustup-init.sh && \
    sh /rustup-init.sh -y && \
    rm /rustup-init.sh && \
    . /root/.cargo/env && \
    rustup default nightly
