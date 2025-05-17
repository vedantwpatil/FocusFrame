# Makefile for Screen Recorder project

# Variables for Rust project
RUST_PROJECT_PATH := internal/editing/video-editing-engine/video-effects-processor
RUST_LIB_TARGET_PATH := $(RUST_PROJECT_PATH)/target/release
RUST_LIB_FILENAME := libvideo_effects_processor.a
RUST_LIBRARY_FULL_PATH := $(RUST_LIB_TARGET_PATH)/$(RUST_LIB_FILENAME)

# Variables for Go project
GO_SOURCE_PKG := ./cmd/recorder
GO_BINARY_NAME := screen_recorder
GO_OUTPUT_DIR := bin
GO_OUTPUT_PATH := $(GO_OUTPUT_DIR)/$(GO_BINARY_NAME)
GO_OUTPUT_VIDEOS := output/

# Phony targets (targets that don't represent files)
.PHONY: all compile_rust compile_go compile_all run_go clean

# Default target: Compiles everything
all: compile_all

# Target to compile the Rust library
compile_rust: $(RUST_LIBRARY_FULL_PATH)

$(RUST_LIBRARY_FULL_PATH):
	@echo ">>> Compiling Rust library..."
	@(cd $(RUST_PROJECT_PATH) && cargo build --release)
	@echo ">>> Rust library compiled: $(RUST_LIBRARY_FULL_PATH)"

# Target to compile the Go program
# This depends on the Rust library file to ensure it's built first.
compile_go: $(GO_OUTPUT_PATH)

$(GO_OUTPUT_PATH): $(GO_SOURCE_PKG)/main.go $(RUST_LIBRARY_FULL_PATH)
	@echo ">>> Compiling Go program..."
	@mkdir -p $(GO_OUTPUT_DIR)
	@go build -o $(GO_OUTPUT_PATH) $(GO_SOURCE_PKG)
	@echo ">>> Go program compiled: $(GO_OUTPUT_PATH)"

# Target to compile both Rust and Go
compile_all: $(GO_OUTPUT_PATH) # Depending on Go output will trigger Rust build if necessary

# Target to run the compiled Go program
run_go: $(GO_OUTPUT_PATH)
	@echo ">>> Running Go program: $(GO_OUTPUT_PATH)"
	@./$(GO_OUTPUT_PATH)

# Target to clean build artifacts
clean:
	@echo ">>> Cleaning Rust project..."
	@(cd $(RUST_PROJECT_PATH) && cargo clean)
	@echo ">>> Cleaning Go build artifacts..."
	@rm -f $(GO_OUTPUT_PATH)
	@rmdir $(GO_OUTPUT_DIR) 2>/dev/null || true # Remove bin directory if empty
	@echo ">>> Clean complete."
	@rm -rf $(GO_OUTPUT_VIDEOS)

