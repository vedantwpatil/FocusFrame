package video

/*
#cgo LDFLAGS: -L${SRCDIR}/../../internal/video/video-editing-engine/video-effects-processor/target/release -lvideo_effects_processor
#include "../../internal/video/video-editing-engine/video-effects-processor/include/video_editing_engine.h"
*/
import "C"

import (
	"unsafe"
)

// Serves as a way to call the effect algorithms in rust from go. Here we will prepare the videos into a way that is simple for the rust code to be able to apply the algorithms/video effects

// Go representation matching Rust's CPoint
type GoPoint struct {
	X           float64
	Y           float64
	TimestampMs int64
}

func SmoothCursorPath(rawPoints []GoPoint, tension, friction, mass float64) []GoPoint {
	if len(rawPoints) == 0 {
		return nil
	}

	// 1. Convert Go slice to C-compatible array
	cPoints := make([]C.CPoint, len(rawPoints))
	for i, p := range rawPoints {
		cPoints[i] = C.CPoint{
			x:            C.double(p.X),
			y:            C.double(p.Y),
			timestamp_ms: C.longlong(p.TimestampMs),
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
	//    Prior to Go 1.17, you'd use a different pattern:
	//    header := (*reflect.SliceHeader)(unsafe.Pointer(&goSlice))
	//    header.Data = uintptr(unsafe.Pointer(cSmoothedPath.points))
	//    header.Len = int(cSmoothedPath.len)
	//    header.Cap = int(cSmoothedPath.len)

	var goSmoothedPoints []GoPoint
	if cSmoothedPath.points != nil && cSmoothedPath.len > 0 {
		// Using unsafe.Slice (Go 1.17+)
		cResultSlice := unsafe.Slice(cSmoothedPath.points, cSmoothedPath.len)
		goSmoothedPoints = make([]GoPoint, cSmoothedPath.len)
		for i, cp := range cResultSlice {
			goSmoothedPoints[i] = GoPoint{
				X:           float64(cp.x),
				Y:           float64(cp.y),
				TimestampMs: int64(cp.timestamp_ms),
			}
		}
	}
	return goSmoothedPoints
}
