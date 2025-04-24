FROM ubuntu:20.04

RUN sed -i 's|http://archive|http://ru.archive|g' /etc/apt/sources.list

RUN apt-get update \
  && apt-get install -y  \
    build-essential  \
    curl \
  && rm -rf /var/lib/apt/lists/* \
  && apt-get clean

WORKDIR /tmp

ARG RUST_VERSION=nightly-2024-02-04

RUN curl --proto '=https' --tlsv1.3 https://sh.rustup.rs -sSf | bash -s -- --default-toolchain $RUST_VERSION -y

ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /opt/app

COPY . .

RUN cargo build --release

CMD [ "/opt/app/target/release/fs-cpu-test" ]
