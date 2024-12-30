all: run

run:
	@cargo run --release --quiet

unzip:
	@unzip URIs.zip
	@unzip URIs2.zip

debug:
	@cargo run --keep-going

watch:
	@cargo watch -x run

clean:
	@cargo clean

re: clean all

.PHONY: run debug watch clean