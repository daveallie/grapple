#!/bin/bash

set -e

cd "$(dirname $0)"
cd ..

case `uname -s` in
    Linux)
        FULL_TOOLCHAIN="x86_64-unknown-linux-gnu"
        ;;
    Darwin)
        FULL_TOOLCHAIN="x86_64-apple-darwin"
        ;;
    *)
        FULL_TOOLCHAIN="$(rustup toolchain list | grep default | awk '{print $1}' | cut -d '-' -f2-)"
        ;;
esac

RELEASE_FOLDER="./target/${FULL_TOOLCHAIN}/release"
BINARY_PATH="${RELEASE_FOLDER}/grapple"
ZIP_FILE="grapple-$(git rev-parse --short HEAD)-${FULL_TOOLCHAIN}.tar.gz"
ZIP_PATH="${RELEASE_FOLDER}/${ZIP_FILE}"
PACKAGE_FOLDER="./pkg"
FINAL_ZIP_PATH="${PACKAGE_FOLDER}/${ZIP_FILE}"

rm ${BINARY_PATH} &>/dev/null || true
rm ${ZIP_PATH} &>/dev/null || true
cargo build --release --target ${FULL_TOOLCHAIN}

echo "Before Strip: $(ls -lh ${BINARY_PATH} | awk '{print $5}')"
strip ${BINARY_PATH}
echo " After Strip: $(ls -lh ${BINARY_PATH} | awk '{print $5}')"

tar -zcf ${ZIP_PATH} ${BINARY_PATH}
echo "GZipped File: $(ls -lh ${ZIP_PATH} | awk '{print $5}')"

mkdir -p ${PACKAGE_FOLDER}
mv ${ZIP_PATH} ${FINAL_ZIP_PATH}

echo "Built and zipped to ${FINAL_ZIP_PATH}"
