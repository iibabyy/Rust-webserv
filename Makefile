all: run

run:
	@cargo build --release
	@clear
	@target/release/webServer

unzip:
	@unzip URIs.zip
	@unzip URIs2.zip

debug:
	@cargo build --keep-going
	@target/debug/webServer

watch:
	@cargo watch -q -x run

clean:
	@cargo clean

re: clean all

.PHONY: run debug watch clean