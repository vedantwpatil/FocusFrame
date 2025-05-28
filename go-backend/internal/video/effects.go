package video

import (
	"context"
	"fmt"

	"github.com/vedantwpatil/Screen-Capture/internal/config"
)

// BlurEffect implements the Effect interface
type BlurEffect struct {
	config           *config.Config
	processor        *Processor
	processedSegment VideoSegment
}

func NewBlurEffect(config *config.Config, processor *Processor) *BlurEffect {
	return &BlurEffect{
		config:    config,
		processor: processor,
	}
}

func (e *BlurEffect) Apply(ctx context.Context, input VideoSegment) (VideoSegment, error) {
	if !e.config.Effects.Blur.Enabled {
		return input, nil
	}
	return e.applyBlur(ctx, input, e.config.Effects.Blur.Radius)
}

func (e *BlurEffect) Validate() error {
	if e.config.Effects.Blur.Radius <= 0 {
		return fmt.Errorf("invalid blur radius: %d", e.config.Effects.Blur.Radius)
	}
	return nil
}

func (e *BlurEffect) GetName() string {
	return "blur"
}

func (e *BlurEffect) GetProcessedSegment() VideoSegment {
	return e.processedSegment
}

func (e *BlurEffect) SetProcessedSegment(segment VideoSegment) {
	e.processedSegment = segment
}

func (e *BlurEffect) applyBlur(ctx context.Context, input VideoSegment, radius int) (VideoSegment, error) {
	fmt.Println("Applying blur effect")
	outputPath := fmt.Sprintf("%s_blurred.mp4", input.Path)

	// Create FFmpeg filter for blur
	filter := fmt.Sprintf("boxblur=%d", radius)

	// Apply the filter
	if err := e.processor.ApplyFFmpegFilter(input.Path, outputPath, filter); err != nil {
		return VideoSegment{}, fmt.Errorf("failed to apply blur: %w", err)
	}

	// Store the processed segment
	e.processedSegment = VideoSegment{
		Path:      outputPath,
		StartTime: input.StartTime,
		EndTime:   input.EndTime,
		Metadata:  input.Metadata,
	}

	return e.processedSegment, nil
}

// ZoomEffect implements the Effect interface
type ZoomEffect struct {
	config           *config.Config
	processor        *Processor
	processedSegment VideoSegment
}

func NewZoomEffect(config *config.Config, processor *Processor) *ZoomEffect {
	return &ZoomEffect{
		config:    config,
		processor: processor,
	}
}

func (e *ZoomEffect) Apply(ctx context.Context, input VideoSegment) (VideoSegment, error) {
	if !e.config.Effects.Zoom.Enabled {
		return input, nil
	}
	return e.applyZoom(ctx, input, e.config.Effects.Zoom.Factor)
}

func (e *ZoomEffect) Validate() error {
	if e.config.Effects.Zoom.Factor <= 0 {
		return fmt.Errorf("invalid zoom factor: %f", e.config.Effects.Zoom.Factor)
	}
	return nil
}

func (e *ZoomEffect) GetName() string {
	return "zoom"
}

func (e *ZoomEffect) GetProcessedSegment() VideoSegment {
	return e.processedSegment
}

func (e *ZoomEffect) SetProcessedSegment(segment VideoSegment) {
	e.processedSegment = segment
}

func (e *ZoomEffect) applyZoom(ctx context.Context, input VideoSegment, factor float64) (VideoSegment, error) {
	fmt.Println("Applying zoom effect")
	outputPath := fmt.Sprintf("%s_zoomed.mp4", input.Path)

	// Create FFmpeg filter for zoom
	filter := fmt.Sprintf("zoompan=z='min(zoom+0.0015,%.2f)':d=125", factor)

	// Apply the filter
	if err := e.processor.ApplyFFmpegFilter(input.Path, outputPath, filter); err != nil {
		return VideoSegment{}, fmt.Errorf("failed to apply zoom: %w", err)
	}

	// Store the processed segment
	e.processedSegment = VideoSegment{
		Path:      outputPath,
		StartTime: input.StartTime,
		EndTime:   input.EndTime,
		Metadata:  input.Metadata,
	}

	return e.processedSegment, nil
}

