FROM ubuntu:16.04
RUN apt-get update && \
    apt-get install -y curl ca-certificates gcc libc6-dev && \
    curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain=1.15.1
ENV PATH=/root/.cargo/bin:$PATH
WORKDIR /rust
COPY . /rust
RUN cargo build
RUN cargo test
CMD ./target/debug/underhanded-rs & sleep 5 && ./script.sh && exit
