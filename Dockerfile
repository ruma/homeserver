FROM rust:1.35

ENV PATH="/root/.cargo/bin:$PATH"

RUN rustup component add clippy && \
    rustup component add rustfmt && \
    apt-get update && \
    apt-get -y --no-install-recommends install build-essential ca-certificates curl git libclang-dev libpq-dev libssl-dev openssl pkg-config && \
    curl -LOs https://github.com/jedisct1/libsodium/releases/download/1.0.17/libsodium-1.0.17.tar.gz && \
    tar -zxvf libsodium-1.0.17.tar.gz && \
    (cd libsodium-1.0.17 && ./configure && make && make check && make install) && \
    ldconfig -v && \
    rm -rf libsodium-1.0.17.tar.gz libsodium-1.0.17 && \
    rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/* && \
    mkdir /source

VOLUME ["/source", "/usr/local/cargo/git", "/usr/local/cargo/registry"]
WORKDIR /source
