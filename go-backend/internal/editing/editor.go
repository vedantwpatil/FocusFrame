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
	"strconv"
	"strings"

	vidio "github.com/AlexEidt/Vidio"
	"github.com/vedantwpatil/Screen-Capture/internal/recording"
	"github.com/vedantwpatil/Screen-Capture/internal/tracking"
)

// Constants for zoom effect
const (
	zoomInDurationSeconds  = 0.5 // Duration of zoom-in effect
	zoomOutDurationSeconds = 0.5 // Duration of zoom-out effect
	zoomInFactor           = 1.5 // Zoom-in amount
	zoomOutFactor          = 1.0 // Zoom-out amount (1.0 means no zoom out, > 1 zoom out)
	blurRadius             = 5   // Blur Radius for the box blur effect
	secondsBeforeClick     = 1   // How many seconds before the click should effects happen
)

// Orchestrates FFmpeg commands for video editing
func EditVideoFile(inputFilePath, outputFilePath string, cursorHistory []tracking.CursorPosition, targetFPS float64) {
	video, err := vidio.NewVideo(inputFilePath)
	if err != nil {
		log.Fatalf("Unable to open the screen recorded video at path: %s \n ERROR: %v", inputFilePath, err)
	}
	defer video.Close()

	var clickFrames []int
	// Holds the frame number it was clicked as well as the click timing
	for index := range cursorHistory {
		if cursorHistory[index].ClickTimeStamp != -1 {
			clickFrames = append(clickFrames, int(cursorHistory[index].ClickTimeStamp.Seconds()))
		}
	}
	// Debugging
	fmt.Println(clickFrames)
	resolution, err := recording.GetVideoResolution(inputFilePath)
	if err != nil {
		log.Fatalf("Unable to properly get the video resolution\n Error Message: %v", err)
	}

	// Temporary file list to concatenate
	var segments []string

	// Add the initial segment to file path name
	segments = append(segments, inputFilePath)

	// It is a faster implementation to segment multiple ffmpeg commands together rather than using any type of post processing so we create a directory which stores all the partial video files
	intermediateOutputFilePath := inputFilePath

	fmt.Println("Applying blur effects")
	// Create a temporary directory for segments
	tempDir, err := os.MkdirTemp("", "video_segments")
	if err != nil {
		log.Fatalf("Failed to create temporary directory: %v", err)
	}
	defer os.RemoveAll(tempDir)

	intermediateOutputFilePath, segments = applyBlurEffects(intermediateOutputFilePath, clickFrames, cursorHistory, secondsBeforeClick, resolution, tempDir, segments)

	fmt.Println("Adding zoom in effect")
	intermediateOutputFilePath, segments = applyZoomInEffect(intermediateOutputFilePath, cursorHistory, clickFrames, targetFPS, resolution, tempDir, segments)

	fmt.Println("Adding mouse tracking")

	intermediateOutputFilePath, segments = applyMouseTracking(intermediateOutputFilePath, cursorHistory, targetFPS, resolution, tempDir, segments)

	fmt.Println("Adding zoom out effect")
	intermediateOutputFilePath, segments = applyZoomOutEffect(intermediateOutputFilePath, cursorHistory, clickFrames, targetFPS, resolution, tempDir, segments)

	// TODO: Implement mouse smoothening
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

// TODO: The issue comes from the way extract segments is being used and the different start and stop times being passed in. In order to complete this function we need to fix this issue

// applyBlurEffects applies blur effects using FFmpeg
func applyBlurEffects(inputFilePath string, clickFrames []int, cursorHistory []tracking.CursorPosition, secondsBeforeClick int, resolution string, tempDir string, segments []string) (string, []string) {
	for i, clickFrame := range clickFrames {
		startTime := math.Max(0, float64(clickFrame)-float64(secondsBeforeClick))
		clickTime := float64(clickFrame)

		// Determine the end time for the remaining segment.
		var endTime float64
		if i+1 < len(clickFrames) {
			// If there's a next click frame, set the end time to the next click.
			endTime = float64(clickFrames[i+1])
		} else {
			// If it's the last click frame, set the end time  o zero so we know that we're at the end of the video
			endTime = 0
		}

		// Before blurred segment
		segmentFileName := fmt.Sprintf("%s/segment_%d.mp4", tempDir, i*3)
		err := extractSegment(inputFilePath, 0, startTime, segmentFileName)
		if err != nil {
			log.Fatalf("could not extract segment: %v", err)
		}
		fmt.Println("Extracted relevant segments")
		segments = append(segments, segmentFileName)
		inputFilePath = segmentFileName

		// Add the blurred segment
		blurredSegmentFileName := fmt.Sprintf("%s/segment_%d_blurred.mp4", tempDir, (i*3)+1)
		err = applyBoxBlur(inputFilePath, startTime, clickTime, blurRadius, blurredSegmentFileName)
		if err != nil {
			log.Fatalf("could not blur segment: %v", err)
		}
		segments = append(segments, blurredSegmentFileName)
		inputFilePath = blurredSegmentFileName

		// Remaining segment
		remainingSegmentFileName := fmt.Sprintf("%s/segment_%d.mp4", tempDir, (i*3)+2)
		err = extractSegment(inputFilePath, clickTime, endTime, remainingSegmentFileName)
		if err != nil {
			log.Fatalf("could not extract segment: %v", err)
		}

		segments = append(segments, remainingSegmentFileName)
		inputFilePath = remainingSegmentFileName
	}
	fmt.Println("Finished applying blur effects")
	return inputFilePath, segments
}

// applyZoomInEffect applies zoom-in effect centered on the mouse click position
func applyZoomInEffect(inputFilePath string, cursorHistory []tracking.CursorPosition, clickFrames []int, targetFPS float64, resolution string, tempDir string, segments []string) (string, []string) {
	width, height, err := parseResolution(resolution)
	if err != nil {
		log.Fatalf("Failed to parse resolution: %v", err)
	}

	for i, clickFrame := range clickFrames {
		zoomSegmentFileName := fmt.Sprintf("%s/segment_%d_zoom_in.mp4", tempDir, (i*3)+1)

		// Find the cursor position corresponding to the clickFrame
		cursorX, cursorY := getFrame(cursorHistory, clickFrame)

		// Apply zoom in zoompan filter centered on mouse
		startTime := float64(clickFrame) - zoomInDurationSeconds
		endTime := float64(clickFrame)
		zoomEffect, err := applyZoomPan(resolution, inputFilePath, zoomSegmentFileName, startTime, endTime, 1.0, zoomInFactor, cursorX, cursorY, int16(targetFPS), int16(width), int16(height))
		if err != nil {
			log.Fatalf("Could not apply zoom in effect to segment: %v", err)
		}
		segments = append(segments, zoomEffect)
		inputFilePath = zoomSegmentFileName
	}

	fmt.Println("Finished applying zoom in effects")
	return inputFilePath, segments
}

func applyZoomOutEffect(inputFilePath string, cursorHistory []tracking.CursorPosition, clickFrames []int, targetFPS float64, resolution string, tempDir string, segments []string) (string, []string) {
	width, height, err := parseResolution(resolution)
	if err != nil {
		log.Fatalf("Failed to parse resolution: %v", err)
	}

	for i, clickFrame := range clickFrames {
		zoomOutSegmentFileName := fmt.Sprintf("%s/segment_%d_zoom_out.mp4", tempDir, (i*3)+1)
		// Find the cursor position corresponding to the clickFrame
		cursorX, cursorY := getFrame(cursorHistory, clickFrame)

		// Apply zoom out effect
		startTime := float64(clickFrame)
		endTime := float64(clickFrame) + zoomOutDurationSeconds
		zoomOut, err := applyZoomPan(resolution, inputFilePath, zoomOutSegmentFileName, startTime, endTime, zoomInFactor, 1.0, cursorX, cursorY, int16(targetFPS), int16(width), int16(height))
		if err != nil {
			log.Fatalf("could not apply zoom out effect to segment: %v", err)
		}
		segments = append(segments, zoomOut)
		inputFilePath = zoomOutSegmentFileName
	}
	fmt.Println("Finished applying zoom out effects")
	return inputFilePath, segments
}

func applyMouseTracking(inputFilePath string, cursorHistory []tracking.CursorPosition, targetFPS float64, resolution string, tempDir string, segments []string) (string, []string) {
	// 	for i := range cursorHistory {
	// mouseTrackingSegmentFileName := fmt.Sprintf("%s/segment_%d_mouseTracking.mp4", tempDir, (i*3)+1)

	// TODO: Implement mouse tracking
	// Apply mouse tracking (Temp variables for now until we properly implement mouse smoothening algorithm)
	var mouseTracking string
	// var err error = nil

	// if err != nil {
	// log.Fatalf("could not apply mouse tracking to segment: %v", err)
	//}
	segments = append(segments, mouseTracking)
	// inputFilePath = mouseTrackingSegmentFileName
	// 	}
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

// applyZoomPan applies a zoom and pan effect using FFmpeg's zoompan filter
func applyZoomPan(resolution, inputPath, outputPath string, startTime float64, endTime float64, zoomStartAmount float64, zoomEndAmount float64, targetX, targetY, outputFPS, width, height int16) (string, error) {
	duration := endTime - startTime
	if duration <= 0 {
		return "", fmt.Errorf("end time must be after start time")
	}

	// Calculate the total number of frames for the zoom animation.
	// We can edit this value to edit how smooth the animation is
	totalFrames := int(duration * float64(outputFPS))
	if totalFrames <= 0 {
		// Avoid divide by zero error
		totalFrames = 1
	}

	// Calculate how much the zoom level should change per frame.
	// If zoomStartAmount is already at zoomEndAmount, zoomIncrement will be 0,
	// resulting in a static zoom (pan only).
	zoomIncrement := (zoomEndAmount - zoomStartAmount) / float64(totalFrames)

	// Construct the zoompan filter string.
	// z: Defines the zoom level.
	//    'if(eq(on,0), zoomStartAmount, min(max(zoom+zoomIncrement, 1.0), zoomEndAmount))'
	//    - on: current input frame number (0-indexed).
	//    - if(eq(on,0), zoomStartAmount, ...): On the first frame ('on' equals 0), set zoom to zoomStartAmount.
	//    - min(max(zoom+zoomIncrement, 1.0), zoomEndAmount): For subsequent frames, increment the zoom.
	//      Ensure zoom doesn't go below 1.0 (no zoom out beyond original) or exceed zoomEndAmount.
	//      The `max(..., 1.0)` is important if you are zooming out (zoomEndAmount < zoomStartAmount).
	// x, y: Define the pan position to keep the zoom centered on (targetX, targetY).
	//    'targetX-(iw/zoom/2)' : iw is input width. This expression calculates the top-left
	//                           X coordinate of the viewport.
	// d=1: Output one frame for each input frame, ensuring smooth animation.
	// s=resolution: Sets the output resolution of the zoomed segment.
	// fps=outputFPS: Sets the frame rate for the zoompan filter's output.
	var zoomExpression string
	if zoomIncrement > 0 { // Zooming In
		zoomExpression = fmt.Sprintf("if(eq(on,0),%.6f,min(zoom+%.6f,%.6f))", zoomStartAmount, zoomIncrement, zoomEndAmount)
	} else if zoomIncrement < 0 { // Zooming Out
		zoomExpression = fmt.Sprintf("if(eq(on,0),%.6f,max(zoom+%.6f,%.6f))", zoomStartAmount, zoomIncrement, zoomEndAmount)
	} else { // No zoom change (static zoom or only panning)
		zoomExpression = fmt.Sprintf("%.6f", zoomStartAmount)
	}

	// Dynamic center calculation based on width and height for better zoom positioning
	xExpression := fmt.Sprintf("%d+(iw-iw/zoom)/2", targetX-width/2)
	yExpression := fmt.Sprintf("%d+(ih-ih/zoom)/2", targetY-height/2)

	zoompanFilter := fmt.Sprintf(
		"zoompan=z='%s':x='%s':y='%s':d=1:s=%s:fps=%d",
		zoomExpression,
		xExpression,
		yExpression,
		resolution,
		outputFPS,
	)

	// Build the FFmpeg command.
	// -ss startTime: Seek to the start time in the input.
	// -to endTime: Process up to the end time from the input.
	//              Note: -to is relative to the beginning of the file, not -ss.
	//              Alternatively, use -t duration.
	// -i inputPath: Specifies the input file.
	// -vf zoompanFilter: Applies the constructed zoompan filter.
	// -c:v libx264: Use libx264 video codec for good quality and compatibility.
	// -preset fast: A balance between encoding speed and file size/quality.
	//               Use "ultrafast" for quicker tests, "medium" or "slow" for better quality.
	// -c:a copy: Copy the audio stream without re-encoding.
	// outputPath: The file to save the processed segment to.
	// -y: Overwrite output file if it exists.
	cmdArgs := []string{
		"-y", // Overwrite output file
		"-ss", fmt.Sprintf("%.3f", startTime),
		"-i", inputPath,
		"-t", fmt.Sprintf("%.3f", duration),
		"-vf", zoompanFilter,
		"-c:v", "libx264",
		"-preset", "fast", // "ultrafast", "fast", "medium", "slow"
		"-c:a", "copy",
		outputPath,
	}
	cmd := exec.Command("ffmpeg", cmdArgs...)

	// For debugging: print the command that will be executed.
	fmt.Println("Executing FFmpeg command:", strings.Join(cmd.Args, " "))

	// Execute the command and capture combined output (stdout and stderr).
	output, err := cmd.CombinedOutput()
	if err != nil {
		// If FFmpeg fails, log its output for easier debugging.
		log.Printf("FFmpeg execution failed. Output:\n%s", string(output))
		return "", fmt.Errorf("ffmpeg command failed: %w. Output: %s", err, string(output))
	}

	return outputPath, nil
}

// extractSegment extracts a segment from a video using FFmpeg
func extractSegment(inputPath string, startTime, endTime float64, outputPath string) error {
	// Convert start and end times to string format with specific precision
	startTimeStr := strconv.FormatFloat(startTime, 'f', 3, 64)

	var cmd *exec.Cmd
	// If endTime is a valid value, use -to, otherwise extract to the end of the video
	if endTime > 0 {
		endTimeStr := strconv.FormatFloat(endTime, 'f', 3, 64)
		cmd = exec.Command("ffmpeg",
			"-i", inputPath,
			"-ss", startTimeStr, // Start time
			"-to", endTimeStr, // End time
			"-c", "copy", // Copy all streams without re-encoding
			outputPath,
		)
	} else {
		cmd = exec.Command("ffmpeg",
			"-i", inputPath,
			"-ss", startTimeStr, // Start time
			"-c", "copy", // Copy all streams without re-encoding
			outputPath,
		)
	}

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

// parseResolution extracts width and height from a resolution string (e.g., "1920x1080")
func parseResolution(resolution string) (int, int, error) {
	parts := strings.Split(resolution, "x")
	if len(parts) != 2 {
		return 0, 0, fmt.Errorf("invalid resolution format: %s", resolution)
	}

	widthStr := parts[0]
	heightStr := parts[1]

	width, err := parseUint(widthStr)
	if err != nil {
		return 0, 0, fmt.Errorf("failed to parse width: %w", err)
	}

	height, err := parseUint(heightStr)
	if err != nil {
		return 0, 0, fmt.Errorf("failed to parse height: %w", err)
	}

	return int(width), int(height), nil
}

func parseUint(s string) (uint, error) {
	var result uint
	for _, r := range s {
		if r < '0' || r > '9' {
			return 0, fmt.Errorf("invalid character in number: %q", r)
		}
		result = result*10 + uint(r-'0')
	}
	return result, nil
}

func getFrame(cursorHistory []tracking.CursorPosition, frame int) (int16, int16) {
	return cursorHistory[frame].X, cursorHistory[frame].Y
}
