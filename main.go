package main

import (
	"fmt"
	"image/png"
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
				actualFPS := startRecording(stopChan)
				outputFile := "recording.mp4"

				// Use the actual FPS for encoding
				err := encodeVideo(outputFile, actualFPS)
				if err != nil {
					fmt.Printf("Error creating video: %v\n", err)
				} else {
					fmt.Println("Screen recording completed successfully")
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

// TODO: Need to increase the frame rate of the capturing

// 2 possible implementations, feeding the frames straight into the video encoding pipeline
func startRecording(stopChan chan struct{}) int {
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
		"-y",                                       // Overwrite output file without asking
		"-framerate", fmt.Sprintf("%d", targetFPS), // Input framerate hint (essential for piped input)
		"-f", "image2pipe", // Input format (piped images)
		"-i", "-", // Input source (stdin/pipe)
		"-c:v", "hevc_videotoolbox", // USE HARDWARE HEVC ENCODER on Apple Silicon [1][4][5]
		"-tag:v", "hvc1", // Apple compatibility tag [1]
		// Quality/Bitrate: VideoToolbox often ignores -crf and -preset [2].
		// Let VideoToolbox manage quality/bitrate initially for speed.
		// If needed, experiment with "-b:v <bitrate>" (e.g., "-b:v", "15M" for 15 Mbps).
		"-pix_fmt", "yuv420p", // Standard pixel format for broad compatibility
		"output.mp4", // Output file name
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
		encoder := png.Encoder{
			CompressionLevel: png.BestSpeed,
		}
		for {
			select {
			case <-ticker.C:
				// Capture screenshot
				img, err := screenshot.CaptureRect(bounds)
				if err != nil {
					fmt.Println("Error capturing:", err)
					continue
				}

				// Encode frame to the pipe
				err = encoder.Encode(w, img)
				if err != nil {
					fmt.Printf("Error encoding frame %d: %v\n", frameCount, err)
					return // Exit goroutine if encoding fails
				}

				frameCount++
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

func encodeVideo(output string, frameRate int) error {
	return nil
}
