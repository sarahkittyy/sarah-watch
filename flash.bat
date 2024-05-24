if [%1] == [debug] (
	cargo build
	espflash flash ./target/xtensa-esp32s3-none-elf/debug/sarah-watch -M
) else (
	cargo build --release
	espflash flash ./target/xtensa-esp32s3-none-elf/release/sarah-watch -M
)