package editing

import (
	"fmt"

	"github.com/vedantwpatil/Screen-Capture/internal/tracking"
	"github.com/vedantwpatil/Screen-Capture/internal/video"
)

func ProcessEffect(
	inputVideo string,
	outputVideo string,
	mouseHistory []tracking.CursorPosition,
	frameRate int16,
) error {
	// Progress handler
	progressHandler := func(percent float32) {
		fmt.Printf("\rProcessing: %.1f%%", percent*100)
	}

	err := video.ProcessRecording(
		inputVideo,
		outputVideo,
		mouseHistory,
		frameRate,
		progressHandler,
	)
	if err != nil {
		return fmt.Errorf("video processing failed: %w", err)
	}

	fmt.Println("\nProcessing complete!")
	return nil
}
