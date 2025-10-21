package video

import "github.com/vedantwpatil/Screen-Capture/internal/tracking"

// ProcessRecording applies all video effects to a completed recording
func ProcessRecording(
	inputVideoPath string,
	outputVideoPath string,
	mouseHistory []tracking.CursorPosition,
	frameRate int16,
	progressCallback func(float32),
) error {
	// Set up configuration
	config := DefaultVideoConfig(int32(frameRate))

	// Path to cursor sprite (adjust as needed)
	cursorSpritePath := "internal/video/cursor-sprite.png"

	// Process the video
	return ProcessVideoWithCursor(
		inputVideoPath,
		outputVideoPath,
		cursorSpritePath,
		mouseHistory,
		config,
		progressCallback,
	)
}
