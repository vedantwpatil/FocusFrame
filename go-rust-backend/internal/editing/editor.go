package editing

// Calls the video effects defined in effects.go
import (
	"github.com/vedantwpatil/Screen-Capture/internal/tracking"
	"github.com/vedantwpatil/Screen-Capture/internal/video"
)

func ProcessEffect(mouseHistory []tracking.CursorPosition) {
	// TODO: Dummy values, need to set this up in the config
	video.SmoothCursorPath(mouseHistory, 0.5, 10, 10, 10, 60)
}
