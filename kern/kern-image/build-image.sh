#!/usr/bin/env bash
set -e

cd "${0%/*}"

pwd

docker build -t kern-image-builder .

docker run --privileged --rm -v "$(pwd):/image" -w "/image" -it kern-image-builder ./linux-build-image.sh

