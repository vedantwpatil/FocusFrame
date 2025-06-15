# Focus Frame

A high-performance screen recording application that captures screen content and adds smooth zoom animations on click. Built with Go and Rust for optimal performance, featuring Tauri + Svelte frontend.

## Architecture

The application uses a hybrid architecture:

### Note

Considering switching around the backend since I ended up just creating a go wrapper around a rust video effects engine which I thought was just introducing unnecessary complexity

Current Setup

- **Backend**: Go + Rust
  - Go handles screen recording and application logic
  - Rust processes video effects and cursor path smoothing (Considering switching rust out for go)
  - FFmpeg for high-performance screen capture and video encoding
- **Frontend**: Tauri + Svelte
  - Modern, responsive UI
  - Cross-platform compatibility

## Current Features

- High-performance screen recording using FFmpeg
- Mouse tracking and cursor path smoothing
- Configurable video effects:
  - Blur effects
  - Zoom animations
  - Click tracking
- Cross-platform support (currently optimized for macOS)

## Project Structure

```
.
├── go-rust-backend/         # Backend implementation
│   ├── cmd/                # Command-line interface
│   ├── internal/           # Core functionality
│   │   ├── config/        # Configuration management
│   │   ├── recording/     # Screen recording logic
│   │   ├── tracking/      # Mouse tracking
│   │   └── video/         # Video processing
│   └── makefile           # Build automation
└── tauri-frontend/        # Frontend application
    ├── src/              # Svelte source code
    └── src-tauri/        # Tauri configuration
```

## Dependencies

- Go 1.17+
- Rust (latest stable)
- FFmpeg
- Tauri
- Node.js (for frontend development)

## Building

1. Install dependencies:

   ```bash
   # Backend
   cd go-rust-backend
   make compile_all

   # Frontend
   cd tauri-frontend
   pnpm install
   ```

2. Run the application:

   ```bash
   # Backend
   make run_go

   # Frontend
   pnpm tauri dev
   ```

## Planned Features

- Cursor hiding for static cursor
- Audio recording support
- Webcam integration
- GUI for screen and capture area selection
- Video editing interface
- Customizable video effects

## Hardware Requirements

The software is optimized for modern hardware, particularly tested on macOS with M3 Max chip. Performance may vary based on your system specifications.

