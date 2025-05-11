package editing

/*
#cgo LDFLAGS: -L${SRCDIR}/../../internal/editing/video-editing-engine/video-effects-processor/target/release -lvideo_effects_processor
#include "../../internal/editing/video-editing-engine/video-effects-processor/include/video_editing_engine.h"
*/
import "C"

import (
	"fmt"
	"log"
	"math"
	"os"
	"os/exec"
	"path/filepath"
	"strings"

	vidio "github.com/AlexEidt/Vidio"
	"github.com/vedantwpatil/Screen-Capture/internal/tracking"
)

// Orchestrates FFmpeg commands for video editing
func EditVideoFile(inputFilePath, outputFilePath string, cursorHistory []tracking.CursorPosition, targetFPS float64) {
	video, err := vidio.NewVideo(inputFilePath)
	if err != nil {
		log.Fatalf("Unable to open the screen recorded video at path: %s \n ERROR: %v", inputFilePath, err)
	}
	defer video.Close()

	var clickFrames []int
	for index := range cursorHistory {
		if cursorHistory[index].ClickTimeStamp != -1 {
			clickFrames = append(clickFrames, int(cursorHistory[index].ClickTimeStamp.Seconds()))
		}
	}
	// Debugging
	fmt.Println(clickFrames)

	// Temporary file list to concatenate
	var segments []string

	// Add the initial segment to file path name
	segments = append(segments, inputFilePath)

	// It is a faster implementation to segment multiple ffmpeg commands together rather than using any type of post processing so we create a directory which stores all the partial video files
	intermediateOutputFilePath := inputFilePath

	fmt.Println("Applying blur effects")
	secondsBeforeClick := 2

	// Create a temporary directory for segments
	tempDir, err := os.MkdirTemp("", "video_segments")
	if err != nil {
		log.Fatalf("Failed to create temporary directory: %v", err)
	}
	defer os.RemoveAll(tempDir)

	intermediateOutputFilePath, segments = applyBlurEffects(intermediateOutputFilePath, clickFrames, secondsBeforeClick, targetFPS, tempDir, segments)

	fmt.Println("Adding zoom in effect")
	// TODO: Implement zoom-in logic.  For now, just use the blurred output for the next stage.
	intermediateOutputFilePath, segments = applyZoomInEffect(intermediateOutputFilePath, clickFrames, targetFPS, tempDir, segments)

	fmt.Println("Adding mouse tracking")
	// TODO: Implement mouse tracking
	intermediateOutputFilePath, segments = applyMouseTracking(intermediateOutputFilePath, cursorHistory, targetFPS, tempDir, segments)

	fmt.Println("Adding zoom out effect")
	intermediateOutputFilePath, segments = applyZoomOutEffect(intermediateOutputFilePath, clickFrames, targetFPS, tempDir, segments)

	fmt.Println("Smoothening mouse path")

	// Concatenate the segments
	fmt.Println("Concatenating the segments")
	if len(segments) > 1 {
		err = concatenateSegments(segments, outputFilePath)
		if err != nil {
			log.Fatalf("Failed to concatenate segments: %v", err)
		}
	} else {
		// If there's only one segment, just copy it to the output
		err = os.Rename(segments[0], outputFilePath)
		if err != nil {
			log.Fatalf("Failed to rename single segment to output file: %v", err)
		}
	}

	fmt.Println("Exporting edited file")
}

// applyBlurEffects applies blur effects using FFmpeg
func applyBlurEffects(inputFilePath string, clickFrames []int, secondsBeforeClick int, targetFPS float64, tempDir string, segments []string) (string, []string) {
	for i, clickFrame := range clickFrames {
		startTime := math.Max(0, float64(clickFrame)-float64(secondsBeforeClick*int(targetFPS)))
		endTime := float64(clickFrame)

		// Before blurred segment

		segmentFileName := fmt.Sprintf("%s/segment_%d.mp4", tempDir, i*3)
		err := extractSegment(inputFilePath, 0, startTime, segmentFileName) // Extracts from the last end time to the blur start time
		if err != nil {
			log.Fatalf("could not extract segment: %v", err)
		}
		fmt.Println("Extracted relevant segments")
		segments = append(segments, segmentFileName)
		inputFilePath = segmentFileName

		// Add the blurred segment
		blurredSegmentFileName := fmt.Sprintf("%s/segment_%d_blurred.mp4", tempDir, (i*3)+1)
		err = applyBoxBlur(inputFilePath, startTime, endTime, 10, blurredSegmentFileName)
		if err != nil {
			log.Fatalf("could not blur segment: %v", err)
		}
		segments = append(segments, blurredSegmentFileName)
		inputFilePath = blurredSegmentFileName

		// Remaining segment
		remainingSegmentFileName := fmt.Sprintf("%s/segment_%d.mp4", tempDir, (i*3)+2)
		err = extractSegment(inputFilePath, endTime, math.Inf(1), remainingSegmentFileName) // Extracts from the last end time to the blur start time
		if err != nil {
			log.Fatalf("could not extract segment: %v", err)
		}

		segments = append(segments, remainingSegmentFileName)
		inputFilePath = remainingSegmentFileName
	}
	fmt.Println("Finished applying blur effects")
	return inputFilePath, segments
}

func applyZoomInEffect(inputFilePath string, clickFrames []int, targetFPS float64, tempDir string, segments []string) (string, []string) {
	for i := range clickFrames {
		zoomSegmentFileName := fmt.Sprintf("%s/segment_%d_zoom.mp4", tempDir, (i*3)+1)

		// Apply zoom in zoompan filter centered on mouse
		zoomEffect, err := applyZoomPan(inputFilePath, 2, 5, 1.5, 1.5, zoomSegmentFileName)
		if err != nil {
			log.Fatalf("could not apply zoom in effect to segment: %v", err)
		}
		segments = append(segments, zoomEffect)
		inputFilePath = zoomSegmentFileName
	}
	fmt.Println("Finished applying zoom in effects")
	return inputFilePath, segments
}

func applyZoomOutEffect(inputFilePath string, clickFrames []int, targetFPS float64, tempDir string, segments []string) (string, []string) {
	for i := range clickFrames {
		zoomOutSegmentFileName := fmt.Sprintf("%s/segment_%d_zoomout.mp4", tempDir, (i*3)+1)

		// Apply zoom out effect
		zoomOut, err := applyZoomPan(inputFilePath, 2, 5, 1, 1, zoomOutSegmentFileName)
		if err != nil {
			log.Fatalf("could not apply zoom out effect to segment: %v", err)
		}
		segments = append(segments, zoomOut)
		inputFilePath = zoomOutSegmentFileName
	}
	fmt.Println("Finished applying zoom in effects")
	return inputFilePath, segments
}

func applyMouseTracking(inputFilePath string, cursorHistory []tracking.CursorPosition, targetFPS float64, tempDir string, segments []string) (string, []string) {
	for i := range cursorHistory {
		mouseTrackingSegmentFileName := fmt.Sprintf("%s/segment_%d_mouseTracking.mp4", tempDir, (i*3)+1)

		// Apply mouse tracking
		mouseTracking, err := applyZoomPan(inputFilePath, 2, 5, 1.5, 1.5, mouseTrackingSegmentFileName)
		if err != nil {
			log.Fatalf("could not apply mouse tracking to segment: %v", err)
		}
		segments = append(segments, mouseTracking)
		inputFilePath = mouseTrackingSegmentFileName
	}
	fmt.Println("Finished applying mouse tracking")
	return inputFilePath, segments
}

// applyBoxBlur applies a box blur to a video segment using FFmpeg
func applyBoxBlur(inputPath string, startTime, endTime float64, blurRadius int, outputPath string) error {
	// Convert start and end times to string format
	startTimeStr := fmt.Sprintf("%f", startTime)
	endTimeStr := fmt.Sprintf("%f", endTime)

	cmd := exec.Command("ffmpeg",
		"-i", inputPath,
		"-vf", fmt.Sprintf("boxblur=%d:enable='between(t,%s,%s)'", blurRadius, startTimeStr, endTimeStr),
		"-c:a", "copy", // Copy audio stream without re-encoding
		outputPath,
	)

	// Debugging
	fmt.Println("FFmpeg command:", strings.Join(cmd.Args, " "))

	// Execute the command
	output, err := cmd.CombinedOutput()
	if err != nil {
		log.Printf("FFmpeg output:\n%s", string(output))
		return fmt.Errorf("failed to apply box blur: %w", err)
	}

	return nil
}

func applyZoomPan(inputPath string, startTime, endTime, zoomAmount, zoomEndAmount float64, outputPath string) (string, error) {
	cmd := exec.Command("ffmpeg",
		"-i", inputPath,
		"-vf", fmt.Sprintf("zoompan=z='%f':d=125", zoomAmount),
		"-c:a", "copy", // Copy audio stream without re-encoding
		outputPath,
	)

	// Debugging
	fmt.Println("FFmpeg command:", strings.Join(cmd.Args, " "))

	// Execute the command
	output, err := cmd.CombinedOutput()
	if err != nil {
		log.Printf("FFmpeg output:\n%s", string(output))
		return "", fmt.Errorf("failed to apply zoom pan: %w", err)
	}

	return outputPath, nil
}

func extractSegment(inputPath string, startTime, endTime float64, outputPath string) error {
	// Convert start and end times to string format
	startTimeStr := fmt.Sprintf("%f", startTime)
	endTimeStr := fmt.Sprintf("%f", endTime)

	cmd := exec.Command("ffmpeg",
		"-i", inputPath,
		"-ss", startTimeStr, // Start time
		"-to", endTimeStr, // End time
		"-c", "copy", // Copy all streams without re-encoding
		outputPath,
	)

	// Debugging
	fmt.Println("FFmpeg command:", strings.Join(cmd.Args, " "))

	// Execute the command
	output, err := cmd.CombinedOutput()
	if err != nil {
		log.Printf("FFmpeg output:\n%s", string(output))
		return fmt.Errorf("failed to extract segment: %w", err)
	}

	return nil
}

// concatenateSegments concatenates video segments using FFmpeg
func concatenateSegments(segmentPaths []string, outputPath string) error {
	// Create a temporary file listing the segments
	concatListPath, err := createConcatList(segmentPaths)
	if err != nil {
		return fmt.Errorf("failed to create concat list: %w", err)
	}
	defer os.Remove(concatListPath)

	cmd := exec.Command("ffmpeg",
		"-f", "concat",
		"-safe", "0", // Needed for relative paths
		"-i", concatListPath,
		"-c", "copy", // Copy all streams without re-encoding
		outputPath,
	)

	// Debugging
	fmt.Println("FFmpeg command:", strings.Join(cmd.Args, " "))

	// Execute the command
	output, err := cmd.CombinedOutput()
	if err != nil {
		log.Printf("FFmpeg output:\n%s", string(output))
		return fmt.Errorf("failed to concatenate segments: %w", err)
	}

	return nil
}

// createConcatList creates a temporary file with a list of files to concatenate
func createConcatList(segmentPaths []string) (string, error) {
	tmpFile, err := os.CreateTemp("", "concat_list.txt")
	if err != nil {
		return "", err
	}
	defer tmpFile.Close()

	for _, path := range segmentPaths {
		// Use absolute paths for safety
		absPath, err := filepath.Abs(path)
		if err != nil {
			return "", fmt.Errorf("failed to get absolute path for %s: %w", path, err)
		}
		_, err = fmt.Fprintf(tmpFile, "file '%s'\n", absPath)
		if err != nil {
			return "", err
		}
	}

	return tmpFile.Name(), nil
}
