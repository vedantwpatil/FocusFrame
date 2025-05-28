package video

import (
	"fmt"
	"os"
	"os/exec"

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
		outputPath,
	)

	if err := cmd.Run(); err != nil {
		return fmt.Errorf("failed to apply filter: %w", err)
	}

	return nil
}
