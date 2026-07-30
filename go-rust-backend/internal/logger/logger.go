// Package logger provides leveled, timestamped logging for the recorder backend.
// All levels write to stderr so they never mix with the CLI's stdout UI text.
// Debug is silent unless DEBUG is set in the environment, mirroring the Rust
// side's RUST_LOG-gated verbosity.
package logger

import (
	"io"
	"log"
	"os"
)

const flags = log.Ldate | log.Ltime

var (
	Info  = log.New(os.Stderr, "[INFO]  ", flags)
	Warn  = log.New(os.Stderr, "[WARN]  ", flags)
	Error = log.New(os.Stderr, "[ERROR] ", flags)
	Debug = log.New(debugWriter(), "[DEBUG] ", flags)
)

func debugWriter() io.Writer {
	if os.Getenv("DEBUG") != "" {
		return os.Stderr
	}
	return io.Discard
}
