package video

import (
	"context"
	"fmt"
	"math"
	"os"
	"sort"
	"strings"
	"time"

	"github.com/vedantwpatil/Screen-Capture/internal/config"
	"github.com/vedantwpatil/Screen-Capture/internal/tracking"
)

// ClickEvent represents a mouse click with position and timing
type ClickEvent struct {
	X        int
	Y        int
	Time     time.Time
	Duration float64 // Duration in seconds to apply effect
}

// BlurEffect implements the Effect interface
type BlurEffect struct {
	config           *config.Config
	processor        *Processor
	processedSegment VideoSegment
	clicks           []tracking.CursorPosition
}

func NewBlurEffect(config *config.Config, processor *Processor) *BlurEffect {
	return &BlurEffect{
		config:    config,
		processor: processor,
		clicks:    make([]tracking.CursorPosition, 0),
	}
}

func (e *BlurEffect) AddClickEvent(click tracking.CursorPosition) {
	e.clicks = append(e.clicks, click)
}

func (e *BlurEffect) Apply(ctx context.Context, input VideoSegment) (VideoSegment, error) {
	if !e.config.Effects.Blur.Enabled || len(e.clicks) == 0 {
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
	fmt.Println("Applying motion blur effect")
	outputPath := fmt.Sprintf("%s_blurred.mp4", input.Path)

	// First convert to yuv420p
	tempPath := fmt.Sprintf("%s_temp.mp4", input.Path)
	convertFilter := "[0:v]format=yuv420p[v]"
	if err := e.processor.ApplyFFmpegFilter(input.Path, tempPath, convertFilter); err != nil {
		return VideoSegment{}, fmt.Errorf("failed to convert format: %w", err)
	}

	// Then apply blur effect
	enableExpr := createEnableExpression(e.clicks)
	fmt.Printf("Debug - Enable expression: %s\n", enableExpr)

	blurFilter := fmt.Sprintf(
		"[0:v]boxblur=%d:enable='%s'[v]",
		radius,
		enableExpr,
	)
	fmt.Printf("Debug - Full blur filter: %s\n", blurFilter)

	if err := e.processor.ApplyFFmpegFilter(tempPath, outputPath, blurFilter); err != nil {
		os.Remove(tempPath) // Clean up temp file
		return VideoSegment{}, fmt.Errorf("failed to apply motion blur: %w", err)
	}

	// Clean up temp file
	os.Remove(tempPath)

	// Store the processed segment
	e.processedSegment = VideoSegment{
		Path:      outputPath,
		StartTime: input.StartTime,
		EndTime:   input.EndTime,
		Metadata:  input.Metadata,
	}

	return e.processedSegment, nil
}

// createEnableExpression creates the FFmpeg expression for enabling effects
// It enables effects during cursor movements and around click events
func createEnableExpression(clicks []tracking.CursorPosition) string {
	if len(clicks) == 0 {
		return "0"
	}

	// Create a map to track time ranges and merge overlapping ones
	type TimeRange struct {
		Start float64
		End   float64
	}

	// Use a map to deduplicate and merge overlapping ranges
	ranges := make(map[string]TimeRange)
	preClickTime := 1.0  // 1 second before click
	postClickTime := 2.0 // 2 seconds after click

	// Track cursor movement periods
	lastX, lastY := int16(-1), int16(-1)
	lastTime := time.Duration(0)

	for _, click := range clicks {
		if click.ClickTimeStamp < 0 {
			// This is a cursor position update
			if lastX != -1 && lastY != -1 {
				// Calculate movement speed
				dx := float64(click.X - lastX)
				dy := float64(click.Y - lastY)
				dt := click.ClickTimeStamp.Seconds() - lastTime.Seconds()
				if dt > 0 {
					speed := (dx*dx + dy*dy) / dt

					// Enable blur during fast movements
					if speed > 100 { // Adjust threshold as needed
						startTime := math.Max(0, lastTime.Seconds())
						endTime := click.ClickTimeStamp.Seconds()
						key := fmt.Sprintf("%.3f-%.3f", startTime, endTime)
						ranges[key] = TimeRange{Start: startTime, End: endTime}
					}
				}
			}
			lastX = click.X
			lastY = click.Y
			lastTime = click.ClickTimeStamp
		} else {
			// This is a click event
			startTime := math.Max(0, click.ClickTimeStamp.Seconds()-preClickTime)
			endTime := click.ClickTimeStamp.Seconds() + postClickTime
			key := fmt.Sprintf("%.3f-%.3f", startTime, endTime)
			ranges[key] = TimeRange{Start: startTime, End: endTime}
		}
	}

	// Convert map to slice and sort by start time
	var sortedRanges []TimeRange
	for _, r := range ranges {
		sortedRanges = append(sortedRanges, r)
	}

	// Sort ranges by start time
	sort.Slice(sortedRanges, func(i, j int) bool {
		return sortedRanges[i].Start < sortedRanges[j].Start
	})

	// Merge overlapping ranges
	var mergedRanges []TimeRange
	if len(sortedRanges) > 0 {
		current := sortedRanges[0]
		for i := 1; i < len(sortedRanges); i++ {
			if sortedRanges[i].Start <= current.End {
				// Ranges overlap, merge them
				if sortedRanges[i].End > current.End {
					current.End = sortedRanges[i].End
				}
			} else {
				// No overlap, add current range and start new one
				mergedRanges = append(mergedRanges, current)
				current = sortedRanges[i]
			}
		}
		mergedRanges = append(mergedRanges, current)
	}

	// Create the final expression
	if len(mergedRanges) == 0 {
		return "0"
	}

	var parts []string
	for _, r := range mergedRanges {
		parts = append(parts, fmt.Sprintf("between(t,%f,%f)", r.Start, r.End))
	}

	return strings.Join(parts, "+")
}

// ZoomEffect implements the Effect interface
type ZoomEffect struct {
	config           *config.Config
	processor        *Processor
	processedSegment VideoSegment
	clicks           []tracking.CursorPosition
}

func NewZoomEffect(config *config.Config, processor *Processor) *ZoomEffect {
	return &ZoomEffect{
		config:    config,
		processor: processor,
		clicks:    make([]tracking.CursorPosition, 0),
	}
}

func (e *ZoomEffect) AddClickEvent(click tracking.CursorPosition) {
	e.clicks = append(e.clicks, click)
}

func (e *ZoomEffect) Apply(ctx context.Context, input VideoSegment) (VideoSegment, error) {
	if !e.config.Effects.Zoom.Enabled || len(e.clicks) == 0 {
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
	fmt.Println("Applying cinematic zoom effect")
	outputPath := fmt.Sprintf("%s_zoomed.mp4", input.Path)

	// First convert to yuv420p with a more robust filter chain
	tempPath := fmt.Sprintf("%s_temp.mp4", input.Path)
	convertFilter := "[0:v]format=yuv420p,scale=3840:2160:force_original_aspect_ratio=decrease[v]"
	fmt.Printf("Debug - Convert filter: %s\n", convertFilter)

	if err := e.processor.ApplyFFmpegFilter(input.Path, tempPath, convertFilter); err != nil {
		return VideoSegment{}, fmt.Errorf("failed to convert format: %w", err)
	}

	// Create a simplified zoom filter that follows mouse movement
	var zoomFilter strings.Builder
	zoomFilter.WriteString("[0:v]")

	// Process each click and its surrounding mouse movement
	for i, click := range e.clicks {
		if click.ClickTimeStamp < 0 {
			continue // Skip non-click events
		}

		// Calculate time window for this click
		startTime := math.Max(0, click.ClickTimeStamp.Seconds()-0.5) // 0.5s before click
		clickTime := click.ClickTimeStamp.Seconds()
		endTime := clickTime + 1.0 // 1.0s after click

		if i > 0 {
			zoomFilter.WriteString(",")
		}

		// Create zoom expression for this click
		zoomFilter.WriteString(fmt.Sprintf(
			"zoompan=z='if(between(t,%f,%f),"+
				"if(lt(t,%f),"+
				// Pre-click: Smooth zoom in
				"1+(%f-1)*((t-%f)/%f),"+
				"if(lt(t,%f),"+
				// During click: Peak zoom
				"%f,"+
				// Post-click: Smooth zoom out
				"1+(%f-1)*(1-((t-%f)/%f))"+
				")"+
				")"+
				",1)':"+
				"x='if(between(t,%f,%f),"+
				"if(lt(t,%f),"+
				// Pre-click: Move to click position
				"iw/2+(%d-iw/2)*((t-%f)/%f),"+
				"if(lt(t,%f),"+
				// During click: Exact position
				"%d,"+
				// Post-click: Return to center
				"%d+(iw/2-%d)*(1-((t-%f)/%f))"+
				")"+
				")"+
				",iw/2-(iw/zoom/2))':"+
				"y='if(between(t,%f,%f),"+
				"if(lt(t,%f),"+
				// Pre-click: Move to click position
				"ih/2+(%d-ih/2)*((t-%f)/%f),"+
				"if(lt(t,%f),"+
				// During click: Exact position
				"%d,"+
				// Post-click: Return to center
				"%d+(ih/2-%d)*(1-((t-%f)/%f))"+
				")"+
				")"+
				",ih/2-(ih/zoom/2))':d=1",
			// Zoom parameters
			startTime, endTime,
			clickTime,
			factor, startTime, 0.5,
			clickTime+0.1,
			factor,
			factor, clickTime, 1.0,
			// X position parameters
			startTime, endTime,
			clickTime,
			click.X, startTime, 0.5,
			clickTime+0.1,
			click.X,
			click.X, click.X, clickTime, 1.0,
			// Y position parameters
			startTime, endTime,
			clickTime,
			click.Y, startTime, 0.5,
			clickTime+0.1,
			click.Y,
			click.Y, click.Y, clickTime, 1.0,
		))
	}

	// Add format conversion at the end
	zoomFilter.WriteString(",format=yuv420p,scale=3840:2160:force_original_aspect_ratio=decrease[v]")

	// Debug log the zoom filter
	fmt.Printf("Debug - Zoom filter: %s\n", zoomFilter.String())

	// Apply the zoom effect
	if err := e.processor.ApplyFFmpegFilter(tempPath, outputPath, zoomFilter.String()); err != nil {
		os.Remove(tempPath) // Clean up temp file
		return VideoSegment{}, fmt.Errorf("failed to apply zoom: %w", err)
	}

	// Clean up temp file
	os.Remove(tempPath)

	// Store the processed segment
	e.processedSegment = VideoSegment{
		Path:      outputPath,
		StartTime: input.StartTime,
		EndTime:   input.EndTime,
		Metadata:  input.Metadata,
	}

	return e.processedSegment, nil
}

// createZoomTimeExpression creates the FFmpeg expression for zoom timing
// Includes pre and post-click periods
func createZoomTimeExpression(clicks []tracking.CursorPosition) string {
	expr := "0"
	preClickTime := 1.0  // 1 second before click
	postClickTime := 2.0 // 2 seconds after click

	for i, click := range clicks {
		if click.ClickTimeStamp < 0 {
			continue // Skip non-click events
		}

		startTime := click.ClickTimeStamp.Seconds() - preClickTime
		endTime := click.ClickTimeStamp.Seconds() + postClickTime

		if i > 0 {
			expr += "+"
		}
		expr += fmt.Sprintf("between(t,%f,%f)", startTime, endTime)
	}

	return expr
}

// createZoomExpression creates a physics-based zoom expression
// Uses easing functions for smooth acceleration and deceleration
func createZoomExpression(clicks []tracking.CursorPosition, maxFactor float64) string {
	expr := "1"
	preClickTime := 1.0
	postClickTime := 2.0

	for i, click := range clicks {
		if click.ClickTimeStamp < 0 {
			continue
		}

		startTime := click.ClickTimeStamp.Seconds() - preClickTime
		clickTime := click.ClickTimeStamp.Seconds()
		endTime := clickTime + postClickTime

		if i > 0 {
			expr += "+"
		}

		// Simplified zoom expression for better performance
		expr += fmt.Sprintf(
			"if(between(t,%f,%f),"+
				"if(lt(t,%f),"+
				// Pre-click: Linear ease-in
				"1+(%f-1)*((t-%f)/%f),"+
				"if(lt(t,%f),"+
				// During click: Peak zoom
				"%f,"+
				// Post-click: Linear ease-out
				"1+(%f-1)*(1-((t-%f)/%f))"+
				")"+
				")"+
				",1)",
			startTime, endTime,
			clickTime,
			maxFactor, startTime, preClickTime,
			clickTime+0.1,
			maxFactor,
			maxFactor, clickTime, postClickTime,
		)
	}

	return expr
}

// createCursorXExpression creates a physics-based cursor X position expression
func createCursorXExpression(clicks []tracking.CursorPosition) string {
	expr := "iw/2"
	preClickTime := 1.0
	postClickTime := 2.0

	for i, click := range clicks {
		if click.ClickTimeStamp < 0 {
			continue
		}

		startTime := click.ClickTimeStamp.Seconds() - preClickTime
		clickTime := click.ClickTimeStamp.Seconds()
		endTime := clickTime + postClickTime

		if i > 0 {
			expr += "+"
		}

		// Simplified cursor following for better performance
		expr += fmt.Sprintf(
			"if(between(t,%f,%f),"+
				"if(lt(t,%f),"+
				// Pre-click: Linear ease-in
				"iw/2+(%d-iw/2)*((t-%f)/%f),"+
				"if(lt(t,%f),"+
				// During click: Exact cursor position
				"%d,"+
				// Post-click: Linear ease-out
				"%d+(iw/2-%d)*(1-((t-%f)/%f))"+
				")"+
				")"+
				",iw/2)",
			startTime, endTime,
			clickTime,
			click.X, startTime, preClickTime,
			clickTime+0.1,
			click.X,
			click.X, click.X, clickTime, postClickTime,
		)
	}

	return expr
}

// createCursorYExpression creates a physics-based cursor Y position expression
func createCursorYExpression(clicks []tracking.CursorPosition) string {
	expr := "ih/2"
	preClickTime := 1.0
	postClickTime := 2.0

	for i, click := range clicks {
		if click.ClickTimeStamp < 0 {
			continue
		}

		startTime := click.ClickTimeStamp.Seconds() - preClickTime
		clickTime := click.ClickTimeStamp.Seconds()
		endTime := clickTime + postClickTime

		if i > 0 {
			expr += "+"
		}

		// Simplified cursor following for better performance
		expr += fmt.Sprintf(
			"if(between(t,%f,%f),"+
				"if(lt(t,%f),"+
				// Pre-click: Linear ease-in
				"ih/2+(%d-ih/2)*((t-%f)/%f),"+
				"if(lt(t,%f),"+
				// During click: Exact cursor position
				"%d,"+
				// Post-click: Linear ease-out
				"%d+(ih/2-%d)*(1-((t-%f)/%f))"+
				")"+
				")"+
				",ih/2)",
			startTime, endTime,
			clickTime,
			click.Y, startTime, preClickTime,
			clickTime+0.1,
			click.Y,
			click.Y, click.Y, clickTime, postClickTime,
		)
	}

	return expr
}

// FollowEffect implements the Effect interface
type FollowEffect struct {
	config           *config.Config
	processor        *Processor
	processedSegment VideoSegment
	clicks           []tracking.CursorPosition
}

func NewFollowEffect(config *config.Config, processor *Processor) *FollowEffect {
	return &FollowEffect{
		config:    config,
		processor: processor,
		clicks:    make([]tracking.CursorPosition, 0),
	}
}

func (e *FollowEffect) AddClickEvent(click tracking.CursorPosition) {
	e.clicks = append(e.clicks, click)
}

func (e *FollowEffect) Apply(ctx context.Context, input VideoSegment) (VideoSegment, error) {
	if !e.config.Effects.Follow.Enabled || len(e.clicks) == 0 {
		return input, nil
	}
	return e.applyFollow(ctx, input)
}

func (e *FollowEffect) Validate() error {
	return nil
}

func (e *FollowEffect) GetName() string {
	return "follow"
}

func (e *FollowEffect) GetProcessedSegment() VideoSegment {
	return e.processedSegment
}

func (e *FollowEffect) SetProcessedSegment(segment VideoSegment) {
	e.processedSegment = segment
}

func (e *FollowEffect) applyFollow(ctx context.Context, input VideoSegment) (VideoSegment, error) {
	fmt.Println("Applying mouse follow effect")
	outputPath := fmt.Sprintf("%s_follow.mp4", input.Path)

	// First convert to yuv420p
	tempPath := fmt.Sprintf("%s_temp.mp4", input.Path)
	convertFilter := "[0:v]format=yuv420p[v]"
	if err := e.processor.ApplyFFmpegFilter(input.Path, tempPath, convertFilter); err != nil {
		return VideoSegment{}, fmt.Errorf("failed to convert format: %w", err)
	}

	// Create the follow filter with zoom integration
	var followFilter strings.Builder
	followFilter.WriteString("[0:v]")

	// Process each click and its surrounding mouse movement
	for i, click := range e.clicks {
		if click.ClickTimeStamp < 0 {
			continue // Skip non-click events
		}

		// Calculate time window for this click
		startTime := math.Max(0, click.ClickTimeStamp.Seconds()-e.config.Effects.Follow.Window)
		clickTime := click.ClickTimeStamp.Seconds()
		endTime := clickTime + e.config.Effects.Follow.Window

		if i > 0 {
			followFilter.WriteString(",")
		}

		// Create follow expression for this click with zoom integration
		followFilter.WriteString(fmt.Sprintf(
			"crop=w=iw/2:h=ih/2:"+
				"x='if(between(t,%f,%f),"+
				"if(lt(t,%f),"+
				// Pre-click: Smooth movement to click position
				"iw/4+(%d-iw/2)*((t-%f)/%f),"+
				"if(lt(t,%f),"+
				// During click: Exact position
				"%d,"+
				// Post-click: Smooth return to center
				"%d+(iw/4-%d)*(1-((t-%f)/%f))"+
				")"+
				")"+
				",iw/4)':"+
				"y='if(between(t,%f,%f),"+
				"if(lt(t,%f),"+
				// Pre-click: Smooth movement to click position
				"ih/4+(%d-ih/2)*((t-%f)/%f),"+
				"if(lt(t,%f),"+
				// During click: Exact position
				"%d,"+
				// Post-click: Smooth return to center
				"%d+(ih/4-%d)*(1-((t-%f)/%f))"+
				")"+
				")"+
				",ih/4)',"+
				"scale=iw*2:ih*2[v]",
			// X position parameters
			startTime, endTime,
			clickTime,
			click.X, startTime, e.config.Effects.Follow.Window,
			clickTime+0.1,
			click.X,
			click.X, click.X, clickTime, e.config.Effects.Follow.Window,
			// Y position parameters
			startTime, endTime,
			clickTime,
			click.Y, startTime, e.config.Effects.Follow.Window,
			clickTime+0.1,
			click.Y,
			click.Y, click.Y, clickTime, e.config.Effects.Follow.Window,
		))
	}

	// Add format conversion at the end
	followFilter.WriteString(",format=yuv420p[v]")

	// Debug log the follow filter
	fmt.Printf("Debug - Follow filter: %s\n", followFilter.String())

	// Apply the follow effect
	if err := e.processor.ApplyFFmpegFilter(tempPath, outputPath, followFilter.String()); err != nil {
		os.Remove(tempPath) // Clean up temp file
		return VideoSegment{}, fmt.Errorf("failed to apply follow: %w", err)
	}

	// Clean up temp file
	os.Remove(tempPath)

	// Store the processed segment
	e.processedSegment = VideoSegment{
		Path:      outputPath,
		StartTime: input.StartTime,
		EndTime:   input.EndTime,
		Metadata:  input.Metadata,
	}

	return e.processedSegment, nil
}
