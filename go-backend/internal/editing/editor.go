package editing

/*
#cgo LDFLAGS: -L${SRCDIR}/../../internal/editing/video-editing-engine/video-effects-processor/target/release -lvideo_effects_processor
#include "../../internal/editing/video-editing-engine/video-effects-processor/include/video_editing_engine.h"
*/
import "C"

import (
	"context"
	"fmt"

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

	// Create a progress reporter
	// progress := video.NewProgressReporter()

	// Configure the pipeline
	// e.pipeline.SetProgressReporter(progress)
	// e.pipeline.SetTargetFPS(targetFPS)

	// Add effects to the pipeline
	processor := video.NewProcessor(e.config)
	e.pipeline.AddEffect(video.NewBlurEffect(e.config, processor))
	e.pipeline.AddEffect(video.NewZoomEffect(e.config, processor))

	// Process the video
	if err := e.pipeline.Process(ctx, inputPath, outputPath); err != nil {
		return fmt.Errorf("failed to process video: %w", err)
	}

	return nil
}
