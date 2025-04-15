package main

import (
	"errors"
	"fmt"
	"log"
	"os"
	"os/exec"
	"os/signal"
	"runtime"
	"strconv"
	"strings"
	"sync"
	"syscall"
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
			fmt.Printf("\nReceived signal: %v\n", sig)

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

func startRecording(stopChan chan struct{}) int {
	outputFile := "recording.mp4"
	targetFPS := 60
	var cmd *exec.Cmd

	// Get the OS at runtime
	osType := runtime.GOOS // Use runtime.GOOS here

	fmt.Printf("Detected OS: %s\n", osType)

	// Use the detected OS in the switch statement
	switch osType {
	case "windows":
		fmt.Println("Configuring for Windows...")
		cmd = exec.Command("ffmpeg",
			"-f", "gdigrab", // or "ddagrab"
			"-framerate", fmt.Sprintf("%d", targetFPS),
			"-i", "desktop",
			"-c:v", "libx264", // Choose appropriate encoder
			"-pix_fmt", "yuv420p",
			"-y",
			outputFile)
	case "darwin": // macOS uses "darwin"
		fmt.Println("Configuring for macOS (darwin)...")

		index, err := findScreenDeviceIndex()
		if err != nil {
			fmt.Println("Unable to capture the correct device screen")
		}
		cmd = exec.Command("ffmpeg",
			"-f", "avfoundation",
			"-framerate", fmt.Sprintf("%d", targetFPS),
			// "-pixel_format", "bgr0",
			"-i", index+":none", // Capture screen (Need to update the index with the command ffmpeg -f avfoundation -list_devices true -i "")
			"-c:v", "libx264", // More compatible than hevc_videotoolbox
			"-pix_fmt", "yuv420p", // Uncomment this for compatibility
			"-preset", "ultrafast", // For better performance
			"-y",
			outputFile)
	case "linux":
		fmt.Println("Configuring for Linux...")
		cmd = exec.Command("ffmpeg",
			"-f", "x11grab", // May need PipeWire setup for Wayland: -f pipewire
			"-framerate", fmt.Sprintf("%d", targetFPS),
			"-i", ":0.0", // Or os.Getenv("DISPLAY")
			"-c:v", "libx264",
			"-pix_fmt", "yuv420p",
			"-y",
			outputFile)
	default:
		fmt.Printf("Unsupported operating system: %s\n", osType)
		return 0 // Indicate failure or unhandled OS
	}

	stdinPipe, err := cmd.StdinPipe()
	if err != nil {
		log.Printf("Failed to get stdin pipe: %v", err)
		return 0
	}
	defer stdinPipe.Close()

	cmd.Stderr = os.Stderr

	fmt.Println("Starting FFmpeg...")
	err = cmd.Start()
	if err != nil {
		log.Printf("Failed to start ffmpeg: %v", err) // Log instead of Fatal
		return 0
	}

	// Goroutine to wait for stop signal
	go func() {
		<-stopChan
		fmt.Println("Signaling FFmpeg to stop...")

		_, err := stdinPipe.Write([]byte("q\n"))
		if err != nil {
			fmt.Printf("Failed to write 'q' to the ffmpeg stdin: %v\n", err)
		}
		stdinPipe.Close()
	}()

	// Wait for ffmpeg to finish
	fmt.Println("Waiting for FFmpeg to exit...")
	err = cmd.Wait()

	// Check the exit error after waiting
	if err != nil {
		// Log non-zero exit status, but don't necessarily treat as fatal
		// FFmpeg often exits with status 255 or similar on SIGINT, which is expected
		log.Printf("FFmpeg process finished. Exit status: %v\n", err)
	} else {
		fmt.Println("FFmpeg process finished successfully.")
	}

	// Since ffmpeg controls FPS, return target or indicate success/failure differently
	// Returning targetFPS is a placeholder.
	if err == nil || err.Error() == "signal: interrupt" || err.Error() == "exit status 255" {
		fmt.Println("Recording likely completed.")
		return targetFPS // Or maybe return 1 for success, 0 for failure
	} else {
		fmt.Println("Recording may have failed.")
		return 0 // Indicate failure
	}
}

func encodeVideo(output string, frameRate int) error {
	return nil
}

func findScreenDeviceIndex() (string, error) {
	cmd := exec.Command("ffmpeg", "-f", "avfoundation", "-list_devices", "true", "-i", "")

	// Capture ouput
	outputBytes, err := cmd.CombinedOutput()
	if err != nil {
		if len(outputBytes) == 0 {
			return "", fmt.Errorf("failed to run ffmpeg list_devices command: %v, ouput: %s", err, outputBytes)
		}

		fmt.Println("Ffmpeg list_devices exited non-zero, but produced output. Proceeding with parsing.")
	}

	output := string(outputBytes)
	lines := strings.Split(output, "\n")

	inVideoDevices := false
	videoDeviceIndex := 0
	for _, line := range lines {
		if strings.Contains(line, "AVFoundation video devices:") {
			inVideoDevices = true
			continue
		}
		if strings.Contains(line, "AVFoundation audio devices:") {
			inVideoDevices = false
			break
		}

		if inVideoDevices {

			trimmedLine := strings.TrimSpace(line)
			if strings.Contains(trimmedLine, "Capture screen 0") {
				return strconv.Itoa(videoDeviceIndex), nil
			}

			if strings.Contains(trimmedLine, "]") && len(trimmedLine) > 0 {
				videoDeviceIndex++
			}
		}
	}

	return "", errors.New("could not find 'Capture SCreen 0' in ffmpeg device list")
}
