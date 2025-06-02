package video

/*
#cgo LDFLAGS: -L${SRCDIR}/../../internal/video/video-editing-engine/video-effects-processor/target/release -lvideo_effects_processor
#include "../../internal/video/video-editing-engine/video-effects-processor/include/video_editing_engine.h"
*/
import "C"

import (
	"time"
	"unsafe"

	"github.com/vedantwpatil/Screen-Capture/internal/tracking"
)

func SmoothCursorPath(rawPoints []tracking.CursorPosition, alpha, tension, friction, mass float64, frameRate int16) []tracking.CursorPosition {
	if len(rawPoints) == 0 {
		return nil
	}

	// 1. Convert Go slice to C-compatible array
	cPoints := make([]C.CPoint, len(rawPoints))
	for i, p := range rawPoints {
		// p.ClickTimeStamp is time.Duration (int64 nanoseconds)
		// Rust expects timestamp_ms as f64 (double) representing milliseconds
		timestampMillis := float64(int64(p.ClickTimeStamp)) / 1_000_000.0

		cPoints[i] = C.CPoint{
			x:            C.float(p.X),
			y:            C.float(p.Y),
			timestamp_ms: C.double(timestampMillis),
		}
	}

	numPoints := CalculateFramesInBetweenClicks(rawPoints, frameRate)
	cNumFramesPerSegment := make([]C.int64_t, len(numPoints))
	for i, count := range numPoints {
		cNumFramesPerSegment[i] = C.int64_t(count)
	}

	// 2. Call the Rust function
	cSmoothedPath := C.smooth_cursor_path(
		(*C.CPoint)(unsafe.Pointer(&cPoints[0])),
		C.size_t(len(cPoints)),
		(*C.int64_t)(unsafe.Pointer(&cNumFramesPerSegment[0])),
		C.size_t(len(cNumFramesPerSegment)),
		C.float(alpha),
		C.float(tension),
		C.float(friction),
		C.float(mass),
	)

	// 3. Ensure the memory allocated by Rust is freed eventually
	defer C.free_smoothed_path(cSmoothedPath)

	// 4. Convert the C result back to a Go slice
	var goSmoothedPoints []tracking.CursorPosition
	if cSmoothedPath.points != nil && cSmoothedPath.len > 0 {
		cResultSlice := unsafe.Slice(cSmoothedPath.points, cSmoothedPath.len) // Go 1.17+
		goSmoothedPoints = make([]tracking.CursorPosition, cSmoothedPath.len)
		for i, cp := range cResultSlice {
			// cp.timestamp_ms is C.double representing milliseconds
			// Convert back to time.Duration (nanoseconds)
			timestampNanos := int64(float64(cp.timestamp_ms) * 1_000_000.0)

			goSmoothedPoints[i] = tracking.CursorPosition{
				X:              int16(cp.x), // truncates
				Y:              int16(cp.y), // truncates
				ClickTimeStamp: time.Duration(timestampNanos),
			}
		}
	}
	return goSmoothedPoints
}

func CalculateFramesInBetweenClicks(cursorHistory []tracking.CursorPosition, frameRate int16) []int64 {
	var numFrames []int64

	for i := range len(cursorHistory) - 1 {
		clickTime := cursorHistory[i].ClickTimeStamp
		nextClickTime := cursorHistory[i+1].ClickTimeStamp

		amtTime := nextClickTime - clickTime
		amtFrames := frameRate * int16(amtTime)
		numFrames = append(numFrames, int64(amtFrames))
	}
	return numFrames
}
