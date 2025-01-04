FROM ubuntu:22.04 

RUN apt update && apt install -y curl

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > /rustup-init.sh && \
    bash /rustup-init.sh -y && \
    rm /rustup-init.sh && \
    . /root/.cargo/env && \
    rustup default nightly