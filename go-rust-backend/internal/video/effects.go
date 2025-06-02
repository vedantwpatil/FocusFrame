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

// Serves as a way to call the effect algorithms in rust from go. Here we will prepare the videos into a way that is simple for the rust code to be able to apply the algorithms/video effects

func SmoothCursorPath(rawPoints []tracking.CursorPosition, tension, friction, mass float64) []tracking.CursorPosition {
	if len(rawPoints) == 0 {
		return nil
	}

	// 1. Convert Go slice to C-compatible array
	cPoints := make([]C.CPoint, len(rawPoints))
	for i, p := range rawPoints {
		cPoints[i] = C.CPoint{
			x:            C.double(p.X),
			y:            C.double(p.Y),
			timestamp_ms: C.longlong(int64(p.ClickTimeStamp)),
		}
	}

	// 2. Call the Rust function
	//    The first element's address is passed as a pointer.
	cSmoothedPath := C.smooth_cursor_path((*C.CPoint)(unsafe.Pointer(&cPoints[0])), C.size_t(len(cPoints)), C.double(tension), C.double(friction), C.double(mass))

	// 3. Ensure the memory allocated by Rust is freed eventually
	defer C.free_smoothed_path(cSmoothedPath)

	// 4. Convert the C result back to a Go slice
	//    This requires careful handling of the C pointer and length.
	//    The unsafe.Slice function (Go 1.17+) can be helpful here.

	var goSmoothedPoints []tracking.CursorPosition
	if cSmoothedPath.points != nil && cSmoothedPath.len > 0 {
		// Using unsafe.Slice (Go 1.17+)
		cResultSlice := unsafe.Slice(cSmoothedPath.points, cSmoothedPath.len)
		goSmoothedPoints = make([]tracking.CursorPosition, cSmoothedPath.len)
		for i, cp := range cResultSlice {
			goSmoothedPoints[i] = tracking.CursorPosition{
				X:              int16(cp.x),
				Y:              int16(cp.y),
				ClickTimeStamp: time.Duration(cp.timestamp_ms),
			}
		}
	}
	return goSmoothedPoints
}
