FROM nvcr.io/nvidia/doca/doca:1.5.1-devel

RUN apt-get update
RUN apt install -y curl
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
COPY crates.config /root/.cargo/config
ENV PATH="/root/.cargo/bin:${PATH}"

RUN rustup default nightly
