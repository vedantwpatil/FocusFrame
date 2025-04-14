package main

import (
	"fmt"
	"io"
	"log"
	"os"
	"os/exec"
	"os/signal"
	"sync"
	"syscall"
	"time"

	"github.com/kbinani/screenshot"
)

func timingMain() {
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

// TODO: Need to increase the frame rate of the capturing

// 2 possible implementations, feeding the frames straight into the video encoding pipeline
func testingRecordingSpeed(stopChan chan struct{}) int {
	// Select display to record
	// TODO: Have to create a gui for the user to pick this in the future
	displayIndex := 0
	bounds := screenshot.GetDisplayBounds(displayIndex)

	frameCount := 0
	targetFPS := 30
	ticker := time.NewTicker(time.Second / time.Duration(targetFPS)) // Controls the framerate of the recording
	defer ticker.Stop()

	startTime := time.Now()

	fmt.Printf("Recording screen at target %d FPS ... Press Ctrl+C to stop", targetFPS)

	// Create a pipe to send the images to ffmpeg
	r, w := io.Pipe()

	// Set up ffmpeg command
	cmd := exec.Command("ffmpeg",
		"-framerate", fmt.Sprintf("%d", targetFPS),
		"-f", "rawvideo", // Input format is raw video
		"-pixel_format", "rgba", // **** IMPORTANT: Pixel format is RGBA ****
		"-video_size", fmt.Sprintf("%dx%d", bounds.Dx(), bounds.Dy()), // Explicitly set video size
		"-i", "-", // Input from pipe (stdin)
		"-c:v", "hevc_videotoolbox", // Or h264_videotoolbox, libx264, etc.
		"-pix_fmt", "yuv420p", // Output pixel format for compatibility
		"-y", // **** ADDED: Overwrite output file without asking ****
		"output.mp4",
	)
	cmd.Stderr = os.Stderr

	// Set the pipe as the input to the ffmpeg command
	cmd.Stdin = r

	// Start ffmpeg command
	err := cmd.Start()
	if err != nil {
		log.Fatal(err)
		return 0
	}

	// Function to encode and send frames to ffmpeg
	go func() {
		for {
			select {
			case <-ticker.C:
				loopStartTime := time.Now()

				// --- Measure Screenshot ---
				captureStartTime := time.Now()
				img, err := screenshot.CaptureRect(bounds)
				captureDuration := time.Since(captureStartTime)
				if err != nil {
					fmt.Println("Error capturing:", err)
					continue
				}
				// --------------------------

				// --- Measure Pipe Write ---
				writeStartTime := time.Now()
				_, err = w.Write(img.Pix)
				writeDuration := time.Since(writeStartTime)
				if err != nil {
					// Handle potential broken pipe if ffmpeg exits early
					if err == io.ErrClosedPipe {
						fmt.Println("Pipe closed, likely ffmpeg exited.")
						// Consider stopping the capture loop here
					} else {
						fmt.Println("Error writing pixel data to pipe:", err)
					}
					// Depending on the error, you might want to continue or break
				}
				// -------------------------

				frameCount++
				totalLoopDuration := time.Since(loopStartTime)

				// Log durations periodically or store aggregates
				// Example: Log every 30 frames
				if frameCount%30 == 0 {
					fmt.Printf("Frame %d: Capture=%.2fms, Write=%.2fms, TotalLoop=%.2fms\n",
						frameCount,
						float64(captureDuration.Microseconds())/1000.0,
						float64(writeDuration.Microseconds())/1000.0,
						float64(totalLoopDuration.Microseconds())/1000.0)
				}
			case <-stopChan:
				fmt.Println("Stopping...")
				// Close the writer to signal to ffmpeg that no more data is coming
				if err := w.Close(); err != nil {
					fmt.Println("Error closing pipe:", err)
				}
				return // Exit goroutine
			}
		}
	}()
	// Wait for ffmpeg to finish
	err = cmd.Wait()
	if err != nil {
		log.Fatal(err)
	}
	duration := time.Since(startTime).Abs().Seconds()
	actualFPS := int(float64(frameCount) / duration)

	fmt.Printf("Recording stopped. Captured %d frames in %.2f seconds.\n", frameCount, duration)
	fmt.Printf("Actual average FPS: %d\n", actualFPS)

	return actualFPS
}
