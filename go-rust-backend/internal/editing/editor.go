package editing

/*
#cgo LDFLAGS: -L${SRCDIR}/../../internal/editing/video-editing-engine/video-effects-processor/target/release -lvideo_effects_processor
#include "../../internal/editing/video-editing-engine/video-effects-processor/include/video_editing_engine.h"
*/
import "C"

import (
	"context"
	"fmt"
	"time"

	"github.com/vedantwpatil/Screen-Capture/internal/config"
	"github.com/vedantwpatil/Screen-Capture/internal/tracking"
	"github.com/vedantwpatil/Screen-Capture/internal/video"
)

type Editor struct {
	pipeline *video.Pipeline
	config   *config.Config
}

func NewEditor(config *config.Config) *Editor {
	return &Editor{
		pipeline: video.NewPipeline(config),
		config:   config,
	}
}

func (e *Editor) EditVideo(inputPath, outputPath string, cursorHistory []tracking.CursorPosition, targetFPS float64) error {
	// Create a context for the editing process
	ctx := context.Background()

	// Configure the pipeline
	processor := video.NewProcessor(e.config)
	
	// Create progress bar for overall process
	progressBar := video.NewProgressBar("Processing video effects")
	
	// Add effects to the pipeline in the correct order
	// First apply the follow effect to track cursor movement
	followEffect := video.NewFollowEffect(e.config, processor)
	e.pipeline.AddEffect(followEffect)
	
	// Then apply the zoom effect to emphasize click points
	zoomEffect := video.NewZoomEffect(e.config, processor)
	e.pipeline.AddEffect(zoomEffect)
	
	// Finally apply blur for smooth transitions
	blurEffect := video.NewBlurEffect(e.config, processor)
	e.pipeline.AddEffect(blurEffect)

	// Set mouse events in the pipeline
	e.pipeline.SetMouseEvents(cursorHistory, time.Now())

	// Process the video with progress tracking
	if err := e.pipeline.Process(ctx, inputPath, outputPath); err != nil {
		progressBar.ReportError(err)
		return fmt.Errorf("failed to process video: %w", err)
	}

	// Report completion
	progressBar.ReportComplete()

	return nil
}
