package video

import (
	"context"
	"fmt"
	"os"
	"time"

	"github.com/vedantwpatil/Screen-Capture/internal/config"
	"github.com/vedantwpatil/Screen-Capture/internal/tracking"
)

type Pipeline struct {
	config       *config.Config
	effects      []Effect
	processor    *Processor
	progress     ProgressReporter
	mouseEvents  []tracking.CursorPosition
	startTime    time.Time
}

func NewPipeline(config *config.Config) *Pipeline {
	processor := NewProcessor(config)
	return &Pipeline{
		config:      config,
		effects:     make([]Effect, 0),
		processor:   processor,
		mouseEvents: make([]tracking.CursorPosition, 0),
		startTime:   time.Now(),
	}
}

// SetMouseEvents sets the mouse events for the pipeline
func (p *Pipeline) SetMouseEvents(events []tracking.CursorPosition, startTime time.Time) {
	p.mouseEvents = events
	p.startTime = startTime
}

// AddEffect adds a new effect to the pipeline
func (p *Pipeline) AddEffect(effect Effect) {
	p.effects = append(p.effects, effect)
}

// validateInput checks if the input file exists and is valid
func (p *Pipeline) validateInput(inputPath string) error {
	if _, err := os.Stat(inputPath); os.IsNotExist(err) {
		return fmt.Errorf("input file does not exist: %s", inputPath)
	}
	return nil
}

// processEffect applies a single effect to the video
func (p *Pipeline) processEffect(ctx context.Context, effect Effect, inputPath string) error {
	// Create a video segment from the input
	segment := VideoSegment{
		Path:      inputPath,
		StartTime: float64(p.startTime.Unix()),
		EndTime:   0, // Will be set by the processor
		Metadata:  make(map[string]interface{}),
	}

	// Add click events to the effect
	switch e := effect.(type) {
	case *BlurEffect:
		for _, click := range p.mouseEvents {
			if click.ClickTimeStamp >= 0 { // Only add actual click events
				e.AddClickEvent(click)
			}
		}
	case *ZoomEffect:
		for _, click := range p.mouseEvents {
			if click.ClickTimeStamp >= 0 { // Only add actual click events
				e.AddClickEvent(click)
			}
		}
	case *FollowEffect:
		for _, click := range p.mouseEvents {
			if click.ClickTimeStamp >= 0 { // Only add actual click events
				e.AddClickEvent(click)
			}
		}
	}

	// Create progress bar for this effect
	progressBar := NewProgressBar(fmt.Sprintf("Applying %s effect", effect.GetName()))
	p.progress = progressBar

	// Apply the effect
	processedSegment, err := effect.Apply(ctx, segment)
	if err != nil {
		progressBar.ReportError(err)
		return fmt.Errorf("failed to apply effect %s: %w", effect.GetName(), err)
	}

	// Store the processed segment
	effect.SetProcessedSegment(processedSegment)

	// Report completion
	progressBar.ReportComplete()

	return nil
}

// combineEffects combines all processed effects into the final output
func (p *Pipeline) combineEffects(ctx context.Context, outputPath string) error {
	// Get all processed segments
	segments := make([]VideoSegment, 0)
	for _, effect := range p.effects {
		segments = append(segments, effect.GetProcessedSegment())
	}

	// Combine segments
	if err := p.processor.CombineSegments(segments, outputPath); err != nil {
		return fmt.Errorf("failed to combine effects: %w", err)
	}

	// Report completion
	if p.progress != nil {
		p.progress.ReportComplete()
	}

	return nil
}

// Process applies all effects in the pipeline to the input video
func (p *Pipeline) Process(ctx context.Context, inputPath, outputPath string) error {
	// 1. Validate input
	if err := p.validateInput(inputPath); err != nil {
		return err
	}

	// Create overall progress bar
	progressBar := NewProgressBar("Processing video")
	p.progress = progressBar

	// 2. Process each effect sequentially
	currentInput := inputPath
	tempOutput := ""

	for i, effect := range p.effects {
		// Report progress for overall process
		progress := float64(i) / float64(len(p.effects))
		progressBar.Report(progress)

		// Create a temporary output path for this effect
		if i == len(p.effects)-1 {
			tempOutput = outputPath
		} else {
			tempOutput = fmt.Sprintf("%s_temp_%d.mp4", inputPath[:len(inputPath)-4], i)
		}

		// Apply the effect
		if err := p.processEffect(ctx, effect, currentInput); err != nil {
			// Clean up temporary files
			for j := 0; j < i; j++ {
				tempFile := fmt.Sprintf("%s_temp_%d.mp4", inputPath[:len(inputPath)-4], j)
				os.Remove(tempFile)
			}
			return fmt.Errorf("failed to apply effect %s: %w", effect.GetName(), err)
		}

		// Update input for next effect
		if i < len(p.effects)-1 {
			currentInput = tempOutput
		}

		// Clean up previous temporary file
		if i > 0 {
			prevTemp := fmt.Sprintf("%s_temp_%d.mp4", inputPath[:len(inputPath)-4], i-1)
			os.Remove(prevTemp)
		}
	}

	// Report completion
	progressBar.ReportComplete()

	return nil
}
