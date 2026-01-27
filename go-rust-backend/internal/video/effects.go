package video

/*
#cgo pkg-config: libavcodec libavformat libavutil libswscale libswresample libavfilter libavdevice
#cgo LDFLAGS: -L${SRCDIR}/video-editing-engine/video-effects-processor/target/release -lvideo_effects_processor
#include <stdlib.h>
#include "video-editing-engine/video-effects-processor/include/video_editing_engine.h"

extern void goProgressCallback(float percent);
*/
import "C"

import (
	"fmt"
	"unsafe"

	"github.com/vedantwpatil/Screen-Capture/internal/tracking"
)

var currentProgressHandler func(float32)

//export goProgressCallback
func goProgressCallback(percent C.float) {
	if currentProgressHandler != nil {
		currentProgressHandler(float32(percent))
	}
}

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

	currentProgressHandler = progressHandler

	cInputPath := C.CString(inputVideoPath)
	defer C.free(unsafe.Pointer(cInputPath))

	cOutputPath := C.CString(outputVideoPath)
	defer C.free(unsafe.Pointer(cOutputPath))

	cCursorPath := C.CString(cursorSpritePath)
	defer C.free(unsafe.Pointer(cCursorPath))

	cPoints := make([]C.CPoint, len(mouseHistory))
	for i, p := range mouseHistory {
		timestampMillis := float64(p.ClickTimeStamp.Nanoseconds()) / 1_000_000.0
		cPoints[i] = C.CPoint{
			x:            C.float(p.X),
			y:            C.float(p.Y),
			timestamp_ms: C.double(timestampMillis),
		}
	}

	cConfig := C.VideoProcessingConfig{
		smoothing_alpha: C.float(config.SmoothingAlpha),
		responsiveness:  C.float(config.Responsiveness),
		smoothness:      C.float(config.Smoothness),
		frame_rate:      C.int32_t(config.FrameRate),
		log_level:       C.int32_t(config.LogLevel),
	}

	result := C.process_video_with_cursor(
		cInputPath,
		cOutputPath,
		cCursorPath,
		(*C.CPoint)(unsafe.Pointer(&cPoints[0])),
		C.size_t(len(cPoints)),
		&cConfig,
		C.ProgressCallback(C.goProgressCallback),
	)

	currentProgressHandler = nil

	if result != 0 {
		return fmt.Errorf("video processing failed with error code: %d", result)
	}

	return nil
}

// VideoConfig configures cursor smoothing behavior for video processing.
type VideoConfig struct {
	// SmoothingAlpha is the Catmull-Rom spline parameter (0.5 = centripetal, recommended)
	SmoothingAlpha float64
	// Responsiveness controls how quickly the cursor responds to target changes (0-1)
	// 0.0 = slow, floaty tracking (~400ms settling)
	// 1.0 = snappy, immediate tracking (~60ms settling)
	Responsiveness float64
	// Smoothness controls motion damping (0-1)
	// 0.0 = slight overshoot allowed (zeta=0.7)
	// 1.0 = no overshoot, very smooth (zeta=1.5)
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
