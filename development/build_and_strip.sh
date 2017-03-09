#!/bin/bash

set -e

cd "$(dirname $0)"
cd ..

rm ./target/release/grapple &>/dev/null || true
FULL_TOOLCHAIN="$(rustup toolchain list | grep default | awk '{print $1}' | cut -d '-' -f2-)"
cargo build --release

echo "Before Strip: $(ls -lh ./target/release/grapple | awk '{print $5}')"
strip ./target/release/grapple
echo " After Strip: $(ls -lh ./target/release/grapple | awk '{print $5}')"

VERSION="$(./target/release/grapple -V | awk '{print $2}')"
FILENAME="grapple-$VERSION-$FULL_TOOLCHAIN.tar.gz"

cd ./target/release
tar -zcf $FILENAME ./grapple
echo "GZipped File: $(ls -lh $FILENAME | awk '{print $5}')"

cd ../..
mkdir -p ./pkg
mv ./target/release/$FILENAME ./pkg/

echo "Built and zipped to ./pkg/$FILENAME"
