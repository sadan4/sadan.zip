# see xtask/src/build/qalc_py.rs for the build script
FROM debian:trixie

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
		build-essential \
		pkg-config \
		curl \
		ca-certificates \
		libqalculate-dev \
		python3 \
		python3-dev \
	&& rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/opt/rustup \
	CARGO_HOME=/opt/cargo \
	PATH=/opt/cargo/bin:/usr/local/bin:/usr/bin:/bin
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
		| sh -s -- -y --profile minimal --default-toolchain nightly -c clippy \
	&& chmod -R a+rX /opt/rustup /opt/cargo

WORKDIR /work