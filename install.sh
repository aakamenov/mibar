#!/usr/bin/env bash

cargo build -p bar --release
chmod +x ./target/release/bar
sudo cp ./target/release/bar /usr/bin/mibar
