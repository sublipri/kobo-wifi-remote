#!/bin/sh

# Requires about 10GB of free space
OUTPUT_DIR="${OUTPUT_DIR:-./build}"

mkdir -p "$OUTPUT_DIR"
sudo docker build --build-arg UID="$(id -u)" --build-arg GID="$(id -g)" --tag=kobo-evemu .
sudo docker run -v "$OUTPUT_DIR":/home/kobo/ localhost/kobo-evemu
