#!/bin/bash

echo "🧪 Running Screen Recorder Integration Tests"
echo "============================================"

# 1. Clean build
echo "1️⃣  Cleaning previous builds..."
make clean

# 2. Compile Rust
echo "2️⃣  Compiling Rust library..."
make compile_rust || exit 1

# 3. Verify Rust library
echo "3️⃣  Verifying Rust library..."
make verify_rust || exit 1

# 4. Check cursor sprite
echo "4️⃣  Checking cursor sprite..."
make check_cursor_sprite || exit 1

# 5. Compile Go
echo "5️⃣  Compiling Go application..."
make compile_go || exit 1

# 6. Show build results
echo ""
echo "✅ Build successful!"
echo "📦 Binary: $(ls -lh bin/screen_recorder | awk '{print $9, $5}')"
echo "📚 Rust lib: $(ls -lh internal/video/video-editing-engine/video-effects-processor/target/release/*.a | awk '{print $9, $5}')"
echo ""
echo "🎬 To test manually, run: make run_go"
echo "   Then: 1) Start recording, move mouse & click, Ctrl+C to stop"
echo "         2) Edit video to apply cursor smoothing"
