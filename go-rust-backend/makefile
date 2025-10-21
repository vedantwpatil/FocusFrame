# ============================================================================
# Makefile for Screen Recorder Project
# ============================================================================
# This Makefile manages the build process for a hybrid Go/Rust screen recorder
# with video processing capabilities.
#
# Quick Start:
#   make          - Build everything
#   make dev      - Build and run the application
#   make test     - Run integration tests
#   make clean    - Clean all build artifacts
#   make help     - Show all available commands
# ============================================================================

# ============================================================================
# Variables
# ============================================================================

# Rust library configuration
RUST_PROJECT_PATH := internal/video/video-editing-engine/video-effects-processor
RUST_LIB_TARGET_PATH := $(RUST_PROJECT_PATH)/target/release
RUST_LIB_FILENAME := libvideo_effects_processor.a
RUST_LIBRARY_FULL_PATH := $(RUST_LIB_TARGET_PATH)/$(RUST_LIB_FILENAME)

# Go application configuration
GO_SOURCE_PKG := ./cmd/recorder
GO_BINARY_NAME := screen_recorder
GO_OUTPUT_DIR := bin
GO_OUTPUT_PATH := $(GO_OUTPUT_DIR)/$(GO_BINARY_NAME)

# Output and resources
GO_OUTPUT_VIDEOS := output/
CURSOR_SPRITE_PATH := internal/video/cursor-sprite.png
VIDEO_HEADER_PATH := internal/video/video-editing-engine/video-effects-processor/include/video_editing_engine.h

# Build flags
GO_BUILD_FLAGS := -v
CARGO_BUILD_FLAGS := --release

# Colors for output (optional, for prettier output)
COLOR_RESET := \033[0m
COLOR_BOLD := \033[1m
COLOR_GREEN := \033[32m
COLOR_YELLOW := \033[33m
COLOR_BLUE := \033[34m
COLOR_RED := \033[31m

# ============================================================================
# Phony Targets
# ============================================================================

.PHONY: all help clean rebuild \
        compile_rust compile_go compile_all verify_rust verify_go \
        check_cursor_sprite check_header check_dependencies \
        run_go dev test \
        show_output play_latest clean_videos \
        install_deps

# ============================================================================
# Default Target
# ============================================================================

all: compile_all
	@echo "$(COLOR_GREEN)$(COLOR_BOLD)✅ Build complete!$(COLOR_RESET)"
	@echo "Run '$(COLOR_BLUE)make run_go$(COLOR_RESET)' to start the application"
	@echo "Run '$(COLOR_BLUE)make help$(COLOR_RESET)' to see all available commands"

# ============================================================================
# Help Target
# ============================================================================

help:
	@echo "$(COLOR_BOLD)Screen Recorder - Available Commands$(COLOR_RESET)"
	@echo ""
	@echo "$(COLOR_BOLD)Building:$(COLOR_RESET)"
	@echo "  $(COLOR_BLUE)make$(COLOR_RESET) or $(COLOR_BLUE)make all$(COLOR_RESET)      - Build everything (Rust + Go)"
	@echo "  $(COLOR_BLUE)make compile_rust$(COLOR_RESET)    - Build only the Rust library"
	@echo "  $(COLOR_BLUE)make compile_go$(COLOR_RESET)      - Build only the Go application"
	@echo "  $(COLOR_BLUE)make rebuild$(COLOR_RESET)         - Clean and rebuild everything"
	@echo ""
	@echo "$(COLOR_BOLD)Running:$(COLOR_RESET)"
	@echo "  $(COLOR_BLUE)make run_go$(COLOR_RESET)          - Run the compiled application"
	@echo "  $(COLOR_BLUE)make dev$(COLOR_RESET)             - Build and run in one command"
	@echo "  $(COLOR_BLUE)make test$(COLOR_RESET)            - Run integration tests"
	@echo ""
	@echo "$(COLOR_BOLD)Verification:$(COLOR_RESET)"
	@echo "  $(COLOR_BLUE)make verify_rust$(COLOR_RESET)     - Check if Rust library is built"
	@echo "  $(COLOR_BLUE)make verify_go$(COLOR_RESET)       - Check if Go binary is built"
	@echo "  $(COLOR_BLUE)make check_dependencies$(COLOR_RESET) - Verify all dependencies"
	@echo "  $(COLOR_BLUE)make check_cursor_sprite$(COLOR_RESET) - Verify cursor sprite exists"
	@echo ""
	@echo "$(COLOR_BOLD)Output Management:$(COLOR_RESET)"
	@echo "  $(COLOR_BLUE)make show_output$(COLOR_RESET)     - List all output videos"
	@echo "  $(COLOR_BLUE)make play_latest$(COLOR_RESET)     - Play the most recent processed video"
	@echo "  $(COLOR_BLUE)make clean_videos$(COLOR_RESET)    - Delete all output videos"
	@echo ""
	@echo "$(COLOR_BOLD)Cleanup:$(COLOR_RESET)"
	@echo "  $(COLOR_BLUE)make clean$(COLOR_RESET)           - Remove all build artifacts"
	@echo ""
	@echo "$(COLOR_BOLD)Dependencies:$(COLOR_RESET)"
	@echo "  $(COLOR_BLUE)make install_deps$(COLOR_RESET)    - Install required system dependencies"

# ============================================================================
# Compilation Targets
# ============================================================================

# Compile Rust library
compile_rust: $(RUST_LIBRARY_FULL_PATH)

$(RUST_LIBRARY_FULL_PATH):
	@echo "$(COLOR_YELLOW)>>> Compiling Rust library...$(COLOR_RESET)"
	@(cd $(RUST_PROJECT_PATH) && cargo build $(CARGO_BUILD_FLAGS))
	@if [ -f "$(RUST_LIBRARY_FULL_PATH)" ]; then \
		echo "$(COLOR_GREEN)✅ Rust library compiled: $(RUST_LIBRARY_FULL_PATH)$(COLOR_RESET)"; \
		ls -lh "$(RUST_LIBRARY_FULL_PATH)"; \
	else \
		echo "$(COLOR_RED)❌ Error: Rust library not found after build$(COLOR_RESET)"; \
		exit 1; \
	fi

# Compile Go application
compile_go: $(GO_OUTPUT_PATH)

$(GO_OUTPUT_PATH): $(GO_SOURCE_PKG)/main.go $(RUST_LIBRARY_FULL_PATH)
	@echo "$(COLOR_YELLOW)>>> Compiling Go program...$(COLOR_RESET)"
	@mkdir -p $(GO_OUTPUT_DIR)
	@CGO_ENABLED=1 go build $(GO_BUILD_FLAGS) -o $(GO_OUTPUT_PATH) $(GO_SOURCE_PKG)
	@if [ -f "$(GO_OUTPUT_PATH)" ]; then \
		echo "$(COLOR_GREEN)✅ Go program compiled: $(GO_OUTPUT_PATH)$(COLOR_RESET)"; \
		ls -lh "$(GO_OUTPUT_PATH)"; \
	else \
		echo "$(COLOR_RED)❌ Error: Go binary not found after build$(COLOR_RESET)"; \
		exit 1; \
	fi

# Compile both Rust and Go
compile_all: $(GO_OUTPUT_PATH)
	@echo "$(COLOR_GREEN)$(COLOR_BOLD)✅ All components compiled successfully$(COLOR_RESET)"

# ============================================================================
# Verification Targets
# ============================================================================

# Verify Rust library exists
verify_rust:
	@echo "$(COLOR_YELLOW)>>> Verifying Rust library...$(COLOR_RESET)"
	@if [ -f "$(RUST_LIBRARY_FULL_PATH)" ]; then \
		echo "$(COLOR_GREEN)✅ Rust library found: $(RUST_LIBRARY_FULL_PATH)$(COLOR_RESET)"; \
		ls -lh "$(RUST_LIBRARY_FULL_PATH)"; \
	else \
		echo "$(COLOR_RED)❌ Rust library not found.$(COLOR_RESET)"; \
		echo "   Run '$(COLOR_BLUE)make compile_rust$(COLOR_RESET)' to build it"; \
		exit 1; \
	fi

# Verify Go binary exists
verify_go:
	@echo "$(COLOR_YELLOW)>>> Verifying Go binary...$(COLOR_RESET)"
	@if [ -f "$(GO_OUTPUT_PATH)" ]; then \
		echo "$(COLOR_GREEN)✅ Go binary found: $(GO_OUTPUT_PATH)$(COLOR_RESET)"; \
		ls -lh "$(GO_OUTPUT_PATH)"; \
	else \
		echo "$(COLOR_RED)❌ Go binary not found.$(COLOR_RESET)"; \
		echo "   Run '$(COLOR_BLUE)make compile_go$(COLOR_RESET)' to build it"; \
		exit 1; \
	fi

# Check if cursor sprite exists
check_cursor_sprite:
	@echo "$(COLOR_YELLOW)>>> Checking cursor sprite...$(COLOR_RESET)"
	@if [ ! -f "$(CURSOR_SPRITE_PATH)" ]; then \
		echo "$(COLOR_RED)❌ Error: Cursor sprite not found at $(CURSOR_SPRITE_PATH)$(COLOR_RESET)"; \
		echo "$(COLOR_YELLOW)   Please create a cursor sprite image (32x32 PNG recommended)$(COLOR_RESET)"; \
		echo "   You can download one or create it with:"; \
		echo "   $(COLOR_BLUE)python3 -c \"from PIL import Image, ImageDraw; img = Image.new('RGBA', (32, 32), (0,0,0,0)); draw = ImageDraw.Draw(img); draw.polygon([(0,0), (0,20), (6,15), (10,24), (12,22), (8,13), (16,13)], fill='white', outline='black'); img.save('$(CURSOR_SPRITE_PATH)')\"$(COLOR_RESET)"; \
		exit 1; \
	else \
		echo "$(COLOR_GREEN)✅ Cursor sprite found: $(CURSOR_SPRITE_PATH)$(COLOR_RESET)"; \
		ls -lh "$(CURSOR_SPRITE_PATH)"; \
	fi

# Check if C header exists
check_header:
	@echo "$(COLOR_YELLOW)>>> Checking C header...$(COLOR_RESET)"
	@if [ ! -f "$(VIDEO_HEADER_PATH)" ]; then \
		echo "$(COLOR_RED)❌ Error: C header not found at $(VIDEO_HEADER_PATH)$(COLOR_RESET)"; \
		exit 1; \
	else \
		echo "$(COLOR_GREEN)✅ C header found: $(VIDEO_HEADER_PATH)$(COLOR_RESET)"; \
	fi

# Check all dependencies
check_dependencies:
	@echo "$(COLOR_YELLOW)>>> Checking dependencies...$(COLOR_RESET)"
	@echo ""
	@echo "$(COLOR_BOLD)System Tools:$(COLOR_RESET)"
	@command -v cargo >/dev/null 2>&1 && echo "  $(COLOR_GREEN)✅ Rust/Cargo installed$(COLOR_RESET)" || echo "  $(COLOR_RED)❌ Rust/Cargo not found$(COLOR_RESET)"
	@command -v go >/dev/null 2>&1 && echo "  $(COLOR_GREEN)✅ Go installed: $$(go version)$(COLOR_RESET)" || echo "  $(COLOR_RED)❌ Go not found$(COLOR_RESET)"
	@command -v ffmpeg >/dev/null 2>&1 && echo "  $(COLOR_GREEN)✅ FFmpeg installed: $$(ffmpeg -version | head -n1)$(COLOR_RESET)" || echo "  $(COLOR_RED)❌ FFmpeg not found$(COLOR_RESET)"
	@command -v pkg-config >/dev/null 2>&1 && echo "  $(COLOR_GREEN)✅ pkg-config installed$(COLOR_RESET)" || echo "  $(COLOR_RED)❌ pkg-config not found$(COLOR_RESET)"
	@echo ""
	@echo "$(COLOR_BOLD)FFmpeg Libraries:$(COLOR_RESET)"
	@pkg-config --exists libavcodec && echo "  $(COLOR_GREEN)✅ libavcodec found$(COLOR_RESET)" || echo "  $(COLOR_RED)❌ libavcodec not found$(COLOR_RESET)"
	@pkg-config --exists libavformat && echo "  $(COLOR_GREEN)✅ libavformat found$(COLOR_RESET)" || echo "  $(COLOR_RED)❌ libavformat not found$(COLOR_RESET)"
	@pkg-config --exists libavutil && echo "  $(COLOR_GREEN)✅ libavutil found$(COLOR_RESET)" || echo "  $(COLOR_RED)❌ libavutil not found$(COLOR_RESET)"
	@pkg-config --exists libswscale && echo "  $(COLOR_GREEN)✅ libswscale found$(COLOR_RESET)" || echo "  $(COLOR_RED)❌ libswscale not found$(COLOR_RESET)"
	@echo ""
	@echo "$(COLOR_BOLD)Project Files:$(COLOR_RESET)"
	@$(MAKE) --no-print-directory check_header 2>/dev/null || true
	@$(MAKE) --no-print-directory check_cursor_sprite 2>/dev/null || true

# ============================================================================
# Running Targets
# ============================================================================

# Run the compiled Go program
run_go: $(GO_OUTPUT_PATH)
	@echo "$(COLOR_YELLOW)>>> Running Go program: $(GO_OUTPUT_PATH)$(COLOR_RESET)"
	@echo "$(COLOR_BOLD)Instructions:$(COLOR_RESET)"
	@echo "  1. Choose '1' to start recording"
	@echo "  2. Move mouse and click at least 4 times during recording"
	@echo "  3. Press Ctrl+C to stop recording"
	@echo "  4. Choose '2' to edit/process the video"
	@echo "  5. Choose '3' to exit"
	@echo ""
	@./$(GO_OUTPUT_PATH)

# Development workflow: compile and run
dev: compile_all run_go

# Run integration tests
test: compile_all check_cursor_sprite
	@echo "$(COLOR_YELLOW)>>> Running integration tests...$(COLOR_RESET)"
	@echo "$(COLOR_BOLD)Test Instructions:$(COLOR_RESET)"
	@echo "  - This will start the application"
	@echo "  - Record a short video (5-10 seconds)"
	@echo "  - Move mouse and click at least 4 times"
	@echo "  - Process the video with cursor smoothing"
	@echo ""
	@./$(GO_OUTPUT_PATH)

# ============================================================================
# Output Management Targets
# ============================================================================

# Show all output videos
show_output:
	@echo "$(COLOR_YELLOW)>>> Output videos:$(COLOR_RESET)"
	@if ls $(GO_OUTPUT_VIDEOS)*.mp4 1>/dev/null 2>&1; then \
		ls -lh $(GO_OUTPUT_VIDEOS)*.mp4; \
		echo ""; \
		echo "$(COLOR_BOLD)Summary:$(COLOR_RESET)"; \
		echo "  Total videos: $$(ls $(GO_OUTPUT_VIDEOS)*.mp4 2>/dev/null | wc -l | tr -d ' ')"; \
		echo "  Raw recordings: $$(ls $(GO_OUTPUT_VIDEOS)*[!-edited].mp4 2>/dev/null | grep -v '\-edited' | wc -l | tr -d ' ')"; \
		echo "  Processed videos: $$(ls $(GO_OUTPUT_VIDEOS)*-edited.mp4 2>/dev/null | wc -l | tr -d ' ')"; \
	else \
		echo "$(COLOR_RED)No videos found in $(GO_OUTPUT_VIDEOS)$(COLOR_RESET)"; \
	fi

# Play the most recent processed video (macOS)
play_latest:
	@echo "$(COLOR_YELLOW)>>> Finding latest processed video...$(COLOR_RESET)"
	@LATEST=$$(ls -t $(GO_OUTPUT_VIDEOS)*-edited.mp4 2>/dev/null | head -1); \
	if [ -n "$$LATEST" ]; then \
		echo "$(COLOR_GREEN)Playing: $$LATEST$(COLOR_RESET)"; \
		open "$$LATEST"; \
	else \
		echo "$(COLOR_RED)No processed videos found$(COLOR_RESET)"; \
		echo "Run '$(COLOR_BLUE)make show_output$(COLOR_RESET)' to see all videos"; \
	fi

# Clean all output videos
clean_videos:
	@echo "$(COLOR_YELLOW)>>> Cleaning output videos...$(COLOR_RESET)"
	@if ls $(GO_OUTPUT_VIDEOS)*.mp4 1>/dev/null 2>&1; then \
		rm -f $(GO_OUTPUT_VIDEOS)*.mp4; \
		echo "$(COLOR_GREEN)✅ All videos deleted$(COLOR_RESET)"; \
	else \
		echo "$(COLOR_YELLOW)No videos to clean$(COLOR_RESET)"; \
	fi

# ============================================================================
# Cleanup Targets
# ============================================================================

# Clean all build artifacts
clean:
	@echo "$(COLOR_YELLOW)>>> Cleaning build artifacts...$(COLOR_RESET)"
	@echo "  Cleaning Rust project..."
	@(cd $(RUST_PROJECT_PATH) && cargo clean)
	@echo "  Cleaning Go build artifacts..."
	@rm -f $(GO_OUTPUT_PATH)
	@rmdir $(GO_OUTPUT_DIR) 2>/dev/null || true
	@echo "$(COLOR_GREEN)✅ Clean complete$(COLOR_RESET)"

# Full rebuild from scratch
rebuild: clean compile_all
	@echo "$(COLOR_GREEN)$(COLOR_BOLD)✅ Rebuild complete!$(COLOR_RESET)"

# ============================================================================
# Installation Targets
# ============================================================================

# Install required dependencies (macOS with Homebrew)
install_deps:
	@echo "$(COLOR_YELLOW)>>> Installing dependencies...$(COLOR_RESET)"
	@echo "$(COLOR_BOLD)This will install:$(COLOR_RESET)"
	@echo "  - Rust (via rustup)"
	@echo "  - Go"
	@echo "  - FFmpeg"
	@echo "  - pkg-config"
	@echo ""
	@read -p "Continue? [y/N] " confirm && [ "$$confirm" = "y" ] || exit 1
	@echo ""
	@echo "Installing Rust..."
	@command -v rustup >/dev/null 2>&1 || curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
	@echo "Installing Homebrew packages..."
	@brew install go ffmpeg pkg-config
	@echo "$(COLOR_GREEN)✅ Dependencies installed$(COLOR_RESET)"
	@echo ""
	@echo "Run '$(COLOR_BLUE)make check_dependencies$(COLOR_RESET)' to verify"

# ============================================================================
# Advanced Targets
# ============================================================================

# Format Go code
fmt:
	@echo "$(COLOR_YELLOW)>>> Formatting Go code...$(COLOR_RESET)"
	@go fmt ./...
	@echo "$(COLOR_GREEN)✅ Code formatted$(COLOR_RESET)"

# Lint Rust code
lint_rust:
	@echo "$(COLOR_YELLOW)>>> Linting Rust code...$(COLOR_RESET)"
	@(cd $(RUST_PROJECT_PATH) && cargo clippy)

# Run Go tests
test_go:
	@echo "$(COLOR_YELLOW)>>> Running Go tests...$(COLOR_RESET)"
	@go test -v ./...

# Show build info
info:
	@echo "$(COLOR_BOLD)Build Information$(COLOR_RESET)"
	@echo ""
	@echo "$(COLOR_BOLD)Paths:$(COLOR_RESET)"
	@echo "  Rust project:     $(RUST_PROJECT_PATH)"
	@echo "  Rust library:     $(RUST_LIBRARY_FULL_PATH)"
	@echo "  Go source:        $(GO_SOURCE_PKG)"
	@echo "  Go binary:        $(GO_OUTPUT_PATH)"
	@echo "  Cursor sprite:    $(CURSOR_SPRITE_PATH)"
	@echo "  Output directory: $(GO_OUTPUT_VIDEOS)"
	@echo ""
	@echo "$(COLOR_BOLD)Build Status:$(COLOR_RESET)"
	@test -f "$(RUST_LIBRARY_FULL_PATH)" && echo "  Rust library:  $(COLOR_GREEN)✅ Built$(COLOR_RESET)" || echo "  Rust library:  $(COLOR_RED)❌ Not built$(COLOR_RESET)"
	@test -f "$(GO_OUTPUT_PATH)" && echo "  Go binary:     $(COLOR_GREEN)✅ Built$(COLOR_RESET)" || echo "  Go binary:     $(COLOR_RED)❌ Not built$(COLOR_RESET)"
	@test -f "$(CURSOR_SPRITE_PATH)" && echo "  Cursor sprite: $(COLOR_GREEN)✅ Present$(COLOR_RESET)" || echo "  Cursor sprite: $(COLOR_RED)❌ Missing$(COLOR_RESET)"

# ============================================================================
# End of Makefile
# ============================================================================
