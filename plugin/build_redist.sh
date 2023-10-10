#!/bin/bash -e

#
# Copyright (c) 2023 Flight Level Change Ltd.
#
# All rights reserved.
#

DIR="hints-plugin"
TARGET="${DIR//-/_}"
NAME="FLCHints"
DIST_DIR="../target/dist/${NAME}"
VERSION="$(grep -m1 version ../Cargo.toml | cut -d= -f2 | tr -d ' "')"

(mkdir -p "${DIST_DIR}" && cd "${DIST_DIR}" \
  && mkdir -p lin_x64 mac_x64 win_x64)

for arch in aarch64 x86_64; do
    echo "Building MacOS ($arch)..."
    cargo build --release --target "${arch}-apple-darwin"
done

echo "Creating MacOS universal plugin..."
lipo -create -output "${DIST_DIR}/mac_x64/${NAME}.xpl" \
  "../target/x86_64-apple-darwin/release/lib${TARGET}.dylib" \
  "../target/aarch64-apple-darwin/release/lib${TARGET}.dylib"

for target in x86_64-pc-windows-gnu x86_64-unknown-linux-gnu; do
    echo "Building ${target}..."
    docker run --rm -it \
      --mount type=volume,src=cargo,target=/usr/local/cargo \
      --mount "type=bind,source=${XPLANE_SDK},target=/usr/src/xplane,readonly" \
      --mount "type=bind,source=$(pwd)/../../davionics,target=/usr/src/davionics,readonly" \
      --mount "type=bind,source=$(pwd)/../..,target=/usr/src/myapp" \
      --workdir "/usr/src/myapp/hints/plugin" \
      builder cargo build --release --target "${target}"
done

cp "../target/x86_64-pc-windows-gnu/release/${TARGET}.dll" "${DIST_DIR}/win_x64/${NAME}.xpl"
cp "../target/x86_64-unknown-linux-gnu/release/lib${TARGET}.so" "${DIST_DIR}/lin_x64/${NAME}.xpl"

cp ../LICENSE README.md "${DIST_DIR}"
(cd ../target/dist && zip -rq "${NAME}-${VERSION}.zip" ${NAME})

echo "Distribution built at ../target/dist/${NAME}-${VERSION}.zip"
