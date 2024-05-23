#!/usr/bin/bash

set -e

cargo build --release
espflash flash ./target/xtensa-esp32s3-none-elf/release/sarah-watch -M --list-all-ports -p /dev/ttyACM0
