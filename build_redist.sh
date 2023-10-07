#!/bin/bash -e

DIR="hints-plugin"
TARGET="${DIR//-/_}"
NAME="${DIR%-plugin}"
DIST="target/dist/${NAME}"
VERSION="$(grep -m1 version ${DIR}/Cargo.toml | cut -d= -f2 | tr -d ' "')"

(mkdir -p "${DIST}" && cd "${DIST}" && mkdir -p lin_x64 mac_x64 win_x64)

for arch in aarch64 x86_64; do
    echo "Building MacOS ($arch)..."
    cargo build --release --target "${arch}-apple-darwin"
done

echo "Creating MacOS universal plugin..."
lipo -create -output "${DIST}/mac_x64/${NAME}.xpl" \
  "target/x86_64-apple-darwin/release/lib${TARGET}.dylib" \
  "target/aarch64-apple-darwin/release/lib${TARGET}.dylib"

for target in x86_64-pc-windows-gnu x86_64-unknown-linux-gnu; do
    echo "Building ${target}..."
    docker run --rm -it \
      --mount type=volume,src=cargo,target=/usr/local/cargo \
      --mount "type=bind,source=${XPLANE_SDK},target=/usr/src/xplane,readonly" \
      --mount "type=bind,source=$(pwd)/../davionics,target=/usr/src/davionics,readonly" \
      --mount "type=bind,source=$(pwd),target=/usr/src/myapp" \
      builder cargo build --release --target "${target}"
done

cp "target/x86_64-pc-windows-gnu/release/${TARGET}.dll" "${DIST}/win_x64/${NAME}.xpl"
cp "target/x86_64-unknown-linux-gnu/release/lib${TARGET}.so" "${DIST}/lin_x64/${NAME}.xpl"

(cd target/dist && zip -rq "${DIR}-${VERSION}.zip" "${NAME}")

echo "Distribution built at target/dist/${DIR}-${VERSION}.zip"
