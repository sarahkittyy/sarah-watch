#!/usr/bin/bash

cargo build
sudo espflash flash ./target/xtensa-esp32s3-espidf/debug/sarah-watch -M --list-all-ports -p /dev/ttyACM0
