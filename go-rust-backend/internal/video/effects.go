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
		spring_tension:  C.float(config.SpringTension),
		spring_friction: C.float(config.SpringFriction),
		spring_mass:     C.float(config.SpringMass),
		frame_rate:      C.int32_t(config.FrameRate),
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

type VideoConfig struct {
	SmoothingAlpha float64
	SpringTension  float64
	SpringFriction float64
	SpringMass     float64
	FrameRate      int32
}

func DefaultVideoConfig(frameRate int32) VideoConfig {
	return VideoConfig{
		SmoothingAlpha: 0.5,
		SpringTension:  10.0,
		SpringFriction: 10.0,
		SpringMass:     10.0,
		FrameRate:      frameRate,
	}
}
