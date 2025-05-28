package video

import (
	"context"
	"fmt"
	"os"

	"github.com/vedantwpatil/Screen-Capture/internal/config"
)

type Pipeline struct {
	config    *config.Config
	effects   []Effect
	processor *Processor
	progress  ProgressReporter
}

func NewPipeline(config *config.Config) *Pipeline {
	processor := NewProcessor(config)
	return &Pipeline{
		config:    config,
		effects:   make([]Effect, 0),
		processor: processor,
	}
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
		StartTime: 0,
		EndTime:   0, // Will be set by the processor
		Metadata:  make(map[string]interface{}),
	}

	// Apply the effect
	processedSegment, err := effect.Apply(ctx, segment)
	if err != nil {
		return fmt.Errorf("failed to apply effect %s: %w", effect.GetName(), err)
	}

	// Store the processed segment
	effect.SetProcessedSegment(processedSegment)

	// Update progress
	if p.progress != nil {
		p.progress.Report(0.5) // Example progress value
	}

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

func (p *Pipeline) Process(ctx context.Context, inputPath, outputPath string) error {
	// 1. Validate input
	if err := p.validateInput(inputPath); err != nil {
		return err
	}

	// 2. Process each effect in sequence
	currentInput := inputPath
	for i, effect := range p.effects {
		if err := p.processEffect(ctx, effect, currentInput); err != nil {
			return fmt.Errorf("failed to process effect %d (%s): %w", i, effect.GetName(), err)
		}
		// Use the output of this effect as input for the next effect
		currentInput = effect.GetProcessedSegment().Path
	}

	// 3. Combine effects and produce final output
	return p.combineEffects(ctx, outputPath)
}
