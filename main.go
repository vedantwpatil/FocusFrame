package main

import (
	"fmt"
	"image" // Added for image.RGBA type hint
	"image/png"
	"os"
	"os/signal"
	"sync"
	"syscall"
	"time"

	"github.com/kbinani/screenshot"
	ffmpeg "github.com/u2takey/ffmpeg-go"
)

const (
	targetFPS   = 60            // Target frames per second
	frameBuffer = targetFPS * 2 // Buffer ~2 seconds of frames
)

func main() {
	// Recording state variables
	var (
		isRecording = false
		recordMutex = &sync.Mutex{}
		stopChan    = make(chan struct{}) // To signal producer to stop
		wg          sync.WaitGroup        // To wait for consumer to finish
		frameQueue  chan *image.RGBA      // Channel for frames (the buffer)
	)

	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)

	// Signal handling goroutine remains largely the same
	go func() {
		for sig := range sigChan {
			fmt.Printf("\nReceived signal: %v\n", sig)

			recordMutex.Lock()
			if isRecording {
				fmt.Println("Stopping screen recording...")
				// Signal the producer to stop by closing stopChan
				// Use a non-blocking send or close to avoid deadlock if already closed
				// Closing is generally safer for signaling completion
				select {
				case <-stopChan:
					// Already closed, do nothing
				default:
					close(stopChan)
				}
				isRecording = false

				// **Important**: Don't Unlock Mutex Here Yet!
				// We need to wait for the consumer to finish processing
				// before allowing the main loop or another recording to start.
				// Start a new goroutine to wait and encode, freeing the signal handler.
				go func() {
					fmt.Println("Waiting for frame processing to complete...")
					wg.Wait() // Wait for the consumer goroutine to finish
					fmt.Println("Frame processing finished.")

					// Now perform the encoding
					framesDir := "frames"
					outputFile := "recording.mp4"
					err := encodeVideo(framesDir, outputFile, targetFPS) // Use targetFPS for now
					if err != nil {
						fmt.Printf("Error creating video: %v\n", err)
					} else {
						fmt.Println("Screen recording saved successfully.")
					}
					// Now safe to unlock after waiting and encoding
					recordMutex.Unlock()
				}()

			} else {
				fmt.Println("Exiting application...")
				recordMutex.Unlock() // Unlock before exiting
				os.Exit(0)
			}
			// Unlock is handled differently now for the recording case
			// recordMutex.Unlock() <--- Removed from here
		}
	}()

	// Main menu loop
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

			// --- Setup for new recording ---
			isRecording = true
			stopChan = make(chan struct{})                      // Recreate stop channel for this session
			frameQueue = make(chan *image.RGBA, frameBuffer)    // Create the buffered channel
			if err := os.MkdirAll("frames", 0755); err != nil { // Create frames dir
				fmt.Printf("Error creating frames directory: %v\n", err)
				isRecording = false // Abort start
				recordMutex.Unlock()
				continue
			}
			recordMutex.Unlock() // Unlock *before* starting goroutines

			// --- Start Producer and Consumer ---
			fmt.Println("Starting screen recording... Press Ctrl+C to stop recording.")

			// Add 1 to WaitGroup for the consumer goroutine
			wg.Add(1)

			// Start Producer Goroutine
			go producer(frameQueue, stopChan, targetFPS)

			// Start Consumer Goroutine
			go consumer(frameQueue, &wg)

			// The main loop continues, producer/consumer run in background

		case 2:
			recordMutex.Lock()
			if isRecording {
				// Signal stop if currently recording
				select {
				case <-stopChan:
				default:
					close(stopChan)
				}
				// Wait for consumer and encode (similar to signal handler)
				fmt.Println("Stopping recording before exit...")
				go func() { // Use goroutine to avoid blocking exit command
					wg.Wait()
					fmt.Println("Frame processing finished.")
					err := encodeVideo("frames", "recording_exit.mp4", targetFPS)
					if err != nil {
						fmt.Printf("Error saving video on exit: %v\n", err)
					}
					recordMutex.Unlock() // Unlock after everything is done
					fmt.Println("Exiting...")
					os.Exit(0)
				}()
				// Don't exit immediately, let the goroutine handle cleanup
			} else {
				recordMutex.Unlock() // Unlock if not recording
				fmt.Println("Exiting...")
				return // Exit the main function
			}

		default:
			fmt.Println("Invalid option")
		}
	}
}

// --- Producer Function ---
// Captures frames and sends them to the frameQueue channel
func producer(frameQueue chan<- *image.RGBA, stopChan <-chan struct{}, targetFPS int) {
	// Ensure frameQueue is closed when producer stops, signaling the consumer
	defer close(frameQueue)

	displayIndex := 0 // TODO: Make configurable
	bounds := screenshot.GetDisplayBounds(displayIndex)
	ticker := time.NewTicker(time.Second / time.Duration(targetFPS))
	defer ticker.Stop()

	fmt.Printf("Producer: Capturing screen at target %d FPS...\n", targetFPS)

	captureCount := 0
	startTime := time.Now()

	for {
		select {
		case <-ticker.C:
			img, err := screenshot.CaptureRect(bounds)
			if err != nil {
				fmt.Println("Producer: Error capturing frame:", err)
				continue // Skip this frame
			}

			// Send the captured image pointer to the queue
			// This might block if the queue is full
			select {
			case frameQueue <- img:
				captureCount++
				// Frame sent successfully
			case <-stopChan:
				fmt.Println("Producer: Stop signal received while trying to send frame. Stopping.")
				return
			default:
				// Optional: Log if buffer is full and frame is dropped
				// fmt.Println("Producer: Buffer full, dropping frame!")
			}

		case <-stopChan:
			duration := time.Since(startTime).Seconds()
			actualFPS := 0.0
			if duration > 0 {
				actualFPS = float64(captureCount) / duration
			}
			fmt.Printf("Producer: Stop signal received. Captured %d frames in %.2fs (Avg FPS: %.2f). Stopping capture.\n", captureCount, duration, actualFPS)
			return // Exit the loop and trigger deferred close(frameQueue)
		}
	}
}

// --- Consumer Function ---
// Receives frames from the frameQueue channel and saves them to disk
func consumer(frameQueue <-chan *image.RGBA, wg *sync.WaitGroup) {
	// Signal WaitGroup when this goroutine finishes
	defer wg.Done()

	fmt.Println("Consumer: Ready to process frames...")
	frameCount := 0
	startTime := time.Now()

	// This loop continues as long as frameQueue is open and receiving frames
	// It automatically exits when frameQueue is closed by the producer AND empty
	for img := range frameQueue {
		// Process the received frame (save to PNG)
		fileName := fmt.Sprintf("frames/frame_%05d.png", frameCount)
		file, err := os.Create(fileName)
		if err != nil {
			fmt.Printf("Consumer: Error creating file %s: %v\n", fileName, err)
			continue // Skip this frame if file creation fails
		}

		err = png.Encode(file, img)
		file.Close() // Ensure file is closed even if encode fails

		if err != nil {
			fmt.Printf("Consumer: Error encoding frame %d: %v\n", frameCount, err)
			// Optionally delete the potentially corrupt file: os.Remove(fileName)
			continue // Skip incrementing if encode fails? Or count anyway? Decide based on need.
		}

		frameCount++
		if frameCount%targetFPS == 0 { // Log progress every second (approx)
			fmt.Printf("Consumer: Processed %d frames...\n", frameCount)
		}
	}

	duration := time.Since(startTime).Seconds()
	processingFPS := 0.0
	if duration > 0 {
		processingFPS = float64(frameCount) / duration
	}
	fmt.Printf("Consumer: Finished processing. Processed %d frames in %.2fs (Avg Processing FPS: %.2f).\n", frameCount, duration, processingFPS)
}

// Encodes frames from a directory into a video file and cleans up frames directory
func encodeVideo(framesDir, output string, frameRate int) error {
	// Ensure frameRate is at least 1 to avoid ffmpeg errors
	if frameRate < 1 {
		fmt.Printf("Warning: Calculated frame rate %d is too low, using 1 FPS for encoding.\n", frameRate)
		frameRate = 1
	}

	framePattern := fmt.Sprintf("%s/frame_%%05d.png", framesDir)

	fmt.Printf("Starting video encoding from '%s' to '%s' at %d FPS...\n", framesDir, output, frameRate)

	// Check if frames directory exists and is not empty
	// (Small improvement to avoid running ffmpeg if consumer failed early)
	files, err := os.ReadDir(framesDir)
	if err != nil {
		return fmt.Errorf("cannot read frames directory '%s': %w", framesDir, err)
	}
	if len(files) == 0 {
		return fmt.Errorf("no frames found in directory '%s' to encode", framesDir)
	}

	err = ffmpeg.Input(framePattern, ffmpeg.KwArgs{
		"framerate": frameRate, // Use the provided frame rate
	}).
		Output(output, ffmpeg.KwArgs{
			"c:v": "libx264", // Changed to widely compatible H.264
			// "c:v":     "hevc_videotoolbox", // Keep if only targeting Apple Silicon
			"preset":  "veryfast", // Faster encoding preset
			"crf":     "23",       // Constant Rate Factor (lower means better quality, larger file)
			"pix_fmt": "yuv420p",  // Common pixel format for compatibility
			// Audio flags removed as we are not capturing audio
		}).
		OverWriteOutput(). // Allow overwriting existing output file
		Run()
	if err != nil {
		return fmt.Errorf("encoding failed: %w", err)
	}

	fmt.Println("Video encoding completed successfully.")

	// Clean up the frames directory
	fmt.Printf("Removing frames directory: %s\n", framesDir)
	err = os.RemoveAll(framesDir)
	if err != nil {
		// Log error but don't return it as the video was created successfully
		fmt.Printf("Warning: Failed to clean up frames directory '%s': %v\n", framesDir, err)
	}

	fmt.Printf("Screen recording saved to: %s\n", output)
	return nil
}
