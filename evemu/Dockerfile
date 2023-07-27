from debian:bullseye

RUN apt-get update && apt-get install -y \
# Requirements for koxtoolchain:
build-essential autoconf automake bison flex gawk libtool libtool-bin \
libncurses-dev curl file git gperf help2man texinfo unzip wget \
# Requirements for libevdev:
pkg-config python \
&& rm -rf /var/lib/apt/lists/*
ARG UID=1000
ARG GID=1000
RUN groupadd -g $GID -o kobo
RUN useradd -m -u $UID -g $GID -o -s /bin/bash kobo

COPY --chmod=755 cross-compile.sh /usr/local/bin/cross-compile.sh
USER $UID:$GID
VOLUME /home/kobo
WORKDIR /home/kobo

CMD ["bash", "/usr/local/bin/cross-compile.sh"]
