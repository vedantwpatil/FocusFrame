package video

import (
	"fmt"
	"strings"
	"time"
)

// ProgressBar implements the ProgressReporter interface
type ProgressBar struct {
	total       int
	current     int
	startTime   time.Time
	lastUpdate  time.Time
	description string
}

func NewProgressBar(description string) *ProgressBar {
	return &ProgressBar{
		total:       100,
		current:     0,
		startTime:   time.Now(),
		lastUpdate:  time.Now(),
		description: description,
	}
}

func (p *ProgressBar) Report(progress float64) {
	// Update current progress
	p.current = int(progress * float64(p.total))
	
	// Only update display if enough time has passed (to avoid too frequent updates)
	if time.Since(p.lastUpdate) < 100*time.Millisecond {
		return
	}
	p.lastUpdate = time.Now()
	
	// Calculate percentage
	percentage := float64(p.current) / float64(p.total) * 100
	
	// Calculate elapsed time
	elapsed := time.Since(p.startTime)
	
	// Create progress bar
	barWidth := 30
	completed := int(float64(barWidth) * float64(p.current) / float64(p.total))
	bar := strings.Repeat("=", completed) + strings.Repeat("-", barWidth-completed)
	
	// Print progress
	fmt.Printf("\r%s [%s] %.1f%% Elapsed: %v", 
		p.description,
		bar,
		percentage,
		elapsed.Round(time.Second),
	)
}

func (p *ProgressBar) ReportError(err error) {
	fmt.Printf("\nError: %v\n", err)
}

func (p *ProgressBar) ReportComplete() {
	// Print final progress
	p.Report(1.0)
	fmt.Println() // New line after completion
} 