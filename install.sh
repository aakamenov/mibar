#!/usr/bin/env bash

cargo build --release
chmod +x ./target/release/mibar
sudo cp ./target/release/mibar /usr/bin/mibar
