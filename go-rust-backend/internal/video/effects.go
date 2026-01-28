package video

/*
#cgo pkg-config: libavcodec libavformat libavutil libswscale libswresample libavfilter libavdevice
#cgo LDFLAGS: -L${SRCDIR}/video-editing-engine/video-effects-processor/target/release -lvideo_effects_processor
#include <stdlib.h>
#include "video-editing-engine/video-effects-processor/include/video_editing_engine.h"

// Gateway function that C/Rust can call
extern void goProgressGateway(void *user_data, float percent);
*/
import "C"

import (
	"fmt"
	"runtime/cgo"
	"unsafe"

	"github.com/vedantwpatil/Screen-Capture/internal/tracking"
)

// VideoConfig configures cursor smoothing behavior for video processing.
type VideoConfig struct {
	// SmoothingAlpha is the Catmull-Rom spline parameter (0.5 = centripetal, recommended)
	SmoothingAlpha float64

	// Responsiveness controls the "spring stiffness" of cursor physics (0-1)
	// Maps to: tension = lerp(50.0, 500.0, responsiveness)
	// 0.0 = slow, floaty tracking (~400ms settling time)
	// 1.0 = snappy, immediate tracking (~60ms settling time)
	Responsiveness float64

	// Smoothness controls the "damping" to prevent overshoot (0-1)
	// Maps to: friction = lerp(5.0, 50.0, smoothness)
	// 0.0 = slight overshoot allowed (underdamped, bouncy)
	// 1.0 = no overshoot, critically damped (Screen Studio default)
	Smoothness float64

	// FrameRate is the video frame rate (e.g., 60)
	FrameRate int32

	// LogLevel controls Rust logging verbosity: 0=off, 1=error, 2=warn, 3=info, 4=debug, 5=trace
	LogLevel int32
}

// DefaultVideoConfig returns a balanced configuration for smooth cursor tracking.
func DefaultVideoConfig(frameRate int32) VideoConfig {
	return VideoConfig{
		SmoothingAlpha: 0.5, // Centripetal Catmull-Rom
		Responsiveness: 0.5, // Balanced response time
		Smoothness:     0.7, // Mostly smooth with minimal overshoot
		FrameRate:      frameRate,
		LogLevel:       3, // Info level
	}
}

// The Gateway Function - Exported for C to call
//
//export goProgressGateway
func goProgressGateway(userData unsafe.Pointer, percent C.float) {
	// Safety check: Prevent crash if Rust passes NULL
	if userData == nil {
		return
	}

	// Recover the Go Handle from the void*
	handle := cgo.Handle(userData)

	// Extract the actual channel
	progressChan, ok := handle.Value().(chan float32)
	if !ok {
		// Type assertion failed - this should never happen
		// unless there's a memory corruption bug
		return
	}

	// Non-blocking send to prevent Rust thread from hanging
	select {
	case progressChan <- float32(percent):
	default:
		// Channel full - drop this update
	}
}

// ProcessVideoWithCursor renders a video with smooth cursor overlay.
// This function is thread-safe and can be called concurrently.
func ProcessVideoWithCursor(
	inputVideoPath string,
	outputVideoPath string,
	cursorSpritePath string,
	mouseHistory []tracking.CursorPosition,
	config VideoConfig,
	progressHandler func(float32),
) error {
	if len(mouseHistory) == 0 {
		return fmt.Errorf("no mouse history provided")
	}

	// Convert strings to C strings (heap allocation)
	cInputPath := C.CString(inputVideoPath)
	defer C.free(unsafe.Pointer(cInputPath))

	cOutputPath := C.CString(outputVideoPath)
	defer C.free(unsafe.Pointer(cOutputPath))

	cCursorPath := C.CString(cursorSpritePath)
	defer C.free(unsafe.Pointer(cCursorPath))

	// Debug
	if len(mouseHistory) > 0 {
		first := mouseHistory[0]
		ts := float64(first.ClickTimeStamp.Nanoseconds()) / 1e6
		fmt.Printf("[Go] First Point: X=%.2f Y=%.2f TS=%.4f ms\n", first.X, first.Y, ts)

		if ts < 0 {
			return fmt.Errorf("FATAL: Negative timestamps detected in Go input. Check mouse history capture.")
		}
	}

	// Prepare cursor points
	cPoints := make([]C.CPoint, len(mouseHistory))
	for i, p := range mouseHistory {
		timestampMillis := float64(p.ClickTimeStamp.Nanoseconds()) / 1_000_000.0
		cPoints[i] = C.CPoint{
			x:            C.float(p.X),
			y:            C.float(p.Y),
			timestamp_ms: C.double(timestampMillis),
		}
	}

	// Prepare configuration
	cConfig := C.VideoProcessingConfig{
		smoothing_alpha: C.float(config.SmoothingAlpha),
		responsiveness:  C.float(config.Responsiveness),
		smoothness:      C.float(config.Smoothness),
		frame_rate:      C.int32_t(config.FrameRate),
		log_level:       C.int32_t(config.LogLevel),
	}

	// Create progress channel and pin it with a Handle
	progressChan := make(chan float32, 100)
	handle := cgo.NewHandle(progressChan)
	defer handle.Delete() // CRITICAL: Prevent memory leak

	// Monitor progress in a goroutine
	done := make(chan struct{})
	go func() {
		defer close(done)
		for p := range progressChan {
			if progressHandler != nil {
				progressHandler(p)
			}
		}
	}()

	// Call Rust with the context handle
	result := C.process_video_with_cursor(
		cInputPath,
		cOutputPath,
		cCursorPath,
		(*C.CPoint)(unsafe.Pointer(&cPoints[0])),
		C.size_t(len(cPoints)),
		&cConfig,
		C.ProgressCallback(C.goProgressGateway), // Function pointer
		unsafe.Pointer(handle),                  // Context (the "cookie")
	)

	// Clean up
	close(progressChan)
	<-done // Wait for goroutine to finish

	if result != 0 {
		return fmt.Errorf("video processing failed with error code: %d", result)
	}

	return nil
}
