package video

import (
	"bufio"
	"fmt"
	"os"
	"os/exec"
	"strconv"
	"strings"

	"github.com/vedantwpatil/Screen-Capture/internal/config"
)

type Processor struct {
	config *config.Config
}

func NewProcessor(config *config.Config) *Processor {
	return &Processor{config: config}
}

func (p *Processor) ExtractSegment(inputPath string, startTime, endTime float64) (VideoSegment, error) {
	outputPath := fmt.Sprintf("%s_segment.mp4", inputPath)

	// Use FFmpeg to extract segment
	cmd := exec.Command("ffmpeg",
		"-i", inputPath,
		"-ss", fmt.Sprintf("%.3f", startTime),
		"-to", fmt.Sprintf("%.3f", endTime),
		"-c", "copy",
		"-y",
		outputPath,
	)

	if err := cmd.Run(); err != nil {
		return VideoSegment{}, fmt.Errorf("failed to extract segment: %w", err)
	}

	return VideoSegment{
		Path:      outputPath,
		StartTime: startTime,
		EndTime:   endTime,
		Metadata:  make(map[string]interface{}),
	}, nil
}

func (p *Processor) CombineSegments(segments []VideoSegment, outputPath string) error {
	// Create a temporary file listing the segments
	concatList := ""
	for _, segment := range segments {
		concatList += fmt.Sprintf("file '%s'\n", segment.Path)
	}

	// Write concat list to temporary file
	tmpFile := "concat_list.txt"
	if err := os.WriteFile(tmpFile, []byte(concatList), 0644); err != nil {
		return fmt.Errorf("failed to create concat list: %w", err)
	}
	defer os.Remove(tmpFile)

	// Use FFmpeg to concatenate segments
	cmd := exec.Command("ffmpeg",
		"-f", "concat",
		"-safe", "0",
		"-i", tmpFile,
		"-c", "copy",
		"-y",
		outputPath,
	)

	if err := cmd.Run(); err != nil {
		return fmt.Errorf("failed to combine segments: %w", err)
	}

	return nil
}

func (p *Processor) ApplyFFmpegFilter(inputPath, outputPath, filter string) error {
	cmd := exec.Command("ffmpeg",
		"-i", inputPath,
		"-vf", filter,
		"-c:a", "copy",
		"-y",
		"-progress", "pipe:1",  // Output progress to stdout
		outputPath,
	)

	// Create a pipe to capture FFmpeg's progress output
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return fmt.Errorf("failed to create stdout pipe: %w", err)
	}

	// Start the command
	if err := cmd.Start(); err != nil {
		return fmt.Errorf("failed to start ffmpeg: %w", err)
	}

	// Read progress output
	scanner := bufio.NewScanner(stdout)
	for scanner.Scan() {
		line := scanner.Text()
		if strings.HasPrefix(line, "out_time_ms=") {
			// Parse the time and calculate progress
			timeStr := strings.TrimPrefix(line, "out_time_ms=")
			if timeMs, err := strconv.ParseInt(timeStr, 10, 64); err == nil {
				// Get video duration using ffprobe
				duration, err := p.getVideoDuration(inputPath)
				if err == nil && duration > 0 {
					progress := float64(timeMs) / (duration * 1000000) // Convert to seconds
					if progress > 1.0 {
						progress = 1.0
					}
					// Report progress
					fmt.Printf("\rProgress: %.1f%%", progress*100)
				}
			}
		}
	}

	// Wait for the command to complete
	if err := cmd.Wait(); err != nil {
		return fmt.Errorf("failed to apply filter: %w", err)
	}

	return nil
}

func (p *Processor) getVideoDuration(inputPath string) (float64, error) {
	cmd := exec.Command("ffprobe",
		"-v", "error",
		"-show_entries", "format=duration",
		"-of", "default=noprint_wrappers=1:nokey=1",
		inputPath,
	)

	output, err := cmd.Output()
	if err != nil {
		return 0, fmt.Errorf("failed to get video duration: %w", err)
	}

	duration, err := strconv.ParseFloat(strings.TrimSpace(string(output)), 64)
	if err != nil {
		return 0, fmt.Errorf("failed to parse video duration: %w", err)
	}

	return duration, nil
}
