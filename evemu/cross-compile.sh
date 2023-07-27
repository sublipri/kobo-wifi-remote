#!/bin/bash
#
# Script to cross-compile standalone binaries for evemu-{play,record} using KOReader's Kobo toolchain
#
# Tested in a Debian Bullseye LXC container
#
# Requirements for koxtoolchain:
# apt install build-essential autoconf automake bison flex gawk libtool libtool-bin libncurses-dev curl file git gperf help2man texinfo unzip wget
#
# Requirements for libevdev:
# apt install pkg-config python
#
set -e

KOX_TAG="2021.12"
LIBEVDEV_TAG="libevdev-1.5.9"
EVEMU_TAG="v2.7.0"

if [ ! -d koxtoolchain ]; then
	git clone https://github.com/koreader/koxtoolchain
	cd koxtoolchain
	git checkout "$KOX_TAG"
	./gen-tc.sh kobo
	cd ..
fi

source koxtoolchain/refs/x-compile.sh kobo env

if [ ! -d libevdev ]; then
	git clone https://gitlab.freedesktop.org/libevdev/libevdev.git libevdev
fi
cd libevdev
git checkout "$LIBEVDEV_TAG"
autoreconf -fi
./configure --prefix="$TC_BUILD_DIR" --host="$CROSS_TC" --enable-shared=no --enable-static=yes
make
make install
make clean
cd ..

if [ ! -d evemu ]; then
	git clone https://gitlab.freedesktop.org/libevdev/evemu.git evemu
fi
cd evemu
git checkout "$EVEMU_TAG"
autoreconf -fi
./configure --prefix="$TC_BUILD_DIR" --host="$CROSS_TC" --enable-shared=no --enable-static=yes --disable-python-bindings --disable-tests
make
make install
make clean
cd ..

mkdir -p build/bin build/licenses/{libevdev,evemu}
for b in play record; do
	arm-kobo-linux-gnueabihf-strip --strip-unneeded "${TC_BUILD_DIR}/bin/evemu-${b}"
	cp -t build/bin "${TC_BUILD_DIR}/bin/evemu-${b}"
done
cp -t build/licenses/libevdev libevdev/COPYING
cp -t build/licenses/evemu evemu/COPYING
