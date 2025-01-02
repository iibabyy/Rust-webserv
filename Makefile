# Default target
all: unzip run

# Builds in release mode and runs the application
run:
	@cargo build --release
	@clear
	@./target/release/webserv

# Builds in debug mode and runs the application
debug:
	@cargo build
	@clear
	@./target/debug/webserv

# Builds the project without running it
build:
	@cargo build

# Unzips the URIs files
unzip:
	@unzip -n URIs.zip
	@unzip -n URIs2.zip

# Runs cargo watch command
watch:
	@cargo watch -q -x run

# Cleans the build artifacts
clean:
	@cargo clean

# Cleans and rebuilds everything
re: clean all


.PHONY: all run debug build unzip watch clean re