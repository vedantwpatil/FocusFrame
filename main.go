package main

import (
	"fmt"
	"image/png"
	"os"
	"os/signal"
	"sync"
	"syscall"
	"time"

	"github.com/kbinani/screenshot"
	ffmpeg "github.com/u2takey/ffmpeg-go"
)

func main() {
	// Recording state variables
	var (
		isRecording = false
		recordMutex = &sync.Mutex{}
		stopChan    = make(chan struct{})
	)

	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)

	go func() {
		for sig := range sigChan {
			fmt.Printf("\nRecieved signal: %v\n", sig)

			recordMutex.Lock()
			if isRecording {
				// If we're recording stop recording but don't kill the program
				fmt.Println("Stopped screen recording...")
				close(stopChan)
				isRecording = false
			} else {
				// If we're not recording then we should stop the program
				fmt.Println("Exiting application...")
				recordMutex.Unlock()
				os.Exit(0)
			}
			recordMutex.Unlock()
		}
	}()
	for {
		fmt.Println("\nCommands:")
		fmt.Println("1. Start recording")
		fmt.Println("2. Exit")
		fmt.Print("Choose an option: ")

		var choice int
		fmt.Scanln(&choice)

		switch choice {
		case 1:
			recordMutex.Lock()
			if isRecording {
				fmt.Println("Already recording")
				recordMutex.Unlock()
				continue
			}

			stopChan = make(chan struct{})
			isRecording = true
			recordMutex.Unlock()

			fmt.Println("Starting screen recording... Press Ctrl+C to stop recording.")

			go func() {
				frameCount, actualFPS := startRecording(stopChan)
				if frameCount > 0 {
					framesDir := "frames"
					outputFile := "recording.mp4"

					// Use the actual FPS for encoding
					err := encodeVideo(framesDir, outputFile, actualFPS)
					if err != nil {
						fmt.Printf("Error creating video: %v\n", err)
					} else {
						fmt.Println("Screen recording completed successfully")
					}
				}
			}()

		case 2:
			recordMutex.Lock()
			if isRecording {
				close(stopChan)
			}

			recordMutex.Unlock()
			fmt.Println("Exiting...")
			return

		default:
			fmt.Println("Invalid option")
		}
	}
}

func startRecording(stopChan chan struct{}) (int, int) {
	// Create directory for frames
	os.Mkdir("frames", 0755)

	// Select display to record
	// TODO: Have to create a gui for the user to pick this in the future
	displayIndex := 0
	bounds := screenshot.GetDisplayBounds(displayIndex)

	frameCount := 0
	targetFPS := 60
	ticker := time.NewTicker(time.Second / time.Duration(targetFPS)) // Controls the framerate of the recording
	defer ticker.Stop()

	startTime := time.Now()

	fmt.Printf("Recording screen at target %d FPS ... Press Ctrl+C to stop", targetFPS)

	for {
		select {
		case <-ticker.C:
			// Capture screenshot
			img, err := screenshot.CaptureRect(bounds)
			if err != nil {
				fmt.Println("Error capturing:", err)
				continue
			}

			// Save frame
			fileName := fmt.Sprintf("frames/frame_%05d.png", frameCount)
			file, _ := os.Create(fileName)
			png.Encode(file, img)
			file.Close()

			frameCount++
		case <-stopChan:
			duration := time.Since(startTime).Abs().Seconds()
			actualFPS := int(float64(frameCount) / duration)

			fmt.Printf("Recording stopped. Captured %d frames in %.2f seconds.\n", frameCount, duration)
			fmt.Printf("Actual average FPS: %d\n", actualFPS)

			return frameCount, actualFPS
		}
	}
}

func encodeVideo(framesDir, output string, frameRate int) error {
	framePattern := fmt.Sprintf("%s/frame_%%05d.png", framesDir)

	fmt.Printf("Starting video encoding at %d FPS", frameRate)

	err := ffmpeg.Input(framePattern, ffmpeg.KwArgs{
		"framerate": frameRate, // Specify the frame rate for the input sequence
	}).
		Output(output, ffmpeg.KwArgs{
			"c:v":     "hevc_videotoolbox", // Apple Silicon optimized HEVC encoder
			"crf":     "28",                // Good balance between quality and size
			"tag:v":   "hvc1",              // Proper tag for Apple compatibility
			"preset":  "fast",              // Performance-oriented preset
			"pix_fmt": "yuv420p",           // Compatible pixel format
			"c:a":     "aac",               // Good audio codec (though frames won't have audio)
			"b:a":     "128k",              // Reasonable audio bitrate
		}).
		OverWriteOutput().
		Run()
	if err != nil {
		return fmt.Errorf("encoding failed: %w", err)
	}

	fmt.Println("Video encoding completed successfully")

	// Delete frames directory
	fmt.Printf("Removing frames directory: %s\n", framesDir)
	if err := os.RemoveAll(framesDir); err != nil {
		return fmt.Errorf("failed to clean up frames directory: %w", err)
	}

	fmt.Printf("Screen recording saved to: %s\n", output)
	return nil
}
