#!/usr/bin/bash

set -e

if [[ $1 = "debug" ]]; then
	cargo build
	espflash flash ./target/xtensa-esp32s3-none-elf/debug/sarah-watch -M --list-all-ports -p /dev/ttyACM0
else
	cargo build --release
	espflash flash ./target/xtensa-esp32s3-none-elf/release/sarah-watch -M --list-all-ports -p /dev/ttyACM0
fi
