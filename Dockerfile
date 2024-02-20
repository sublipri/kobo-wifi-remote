ARG CROSS_BASE_IMAGE
FROM $CROSS_BASE_IMAGE

ENV MUSL_TGZ="https://more.musl.cc/10.2.1/x86_64-linux-musl/armv7l-linux-musleabihf-cross.tgz"
ENV PARENT_DIR="/toolchain/"
RUN mkdir -p "$PARENT_DIR"
RUN curl $MUSL_TGZ | tar -xvz -C "$PARENT_DIR"
ENV CROSS_SYSROOT_PATH="$PARENT_DIR/armv7l-linux-musleabihf-cross"
ENV CROSS_INCLUDE_PATH="$CROSS_SYSROOT_PATH/armv7l-linux-musleabihf/include"

RUN dpkg --add-architecture armhf

RUN apt-get update -y && apt-get install -y \
	clang \
	libevdev-dev:armhf \
	python

ENV RUSTFLAGS='-C target-feature=+crt-static'
ENV CC=clang
