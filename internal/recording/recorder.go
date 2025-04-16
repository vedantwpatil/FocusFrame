package recording

import (
	"errors"
	"fmt"
	"log"
	"os"
	"os/exec"
	"runtime"
	"strconv"
	"strings"
)

// Starts recording the user's main screen using ffmpeg to capture the screen and to also encode the video
func StartRecording(stopChan chan struct{}) int {
	outputFile := "recording.mp4"
	targetFPS := 60
	var cmd *exec.Cmd

	// Get the OS at runtime
	osType := runtime.GOOS

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
		return 1 // Or maybe return 1 for success, 0 for failure
	} else {
		fmt.Println("Recording may have failed.")
		return 0 // Indicate failure
	}
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

	// Get proper video device index
	inVideoDevices := false
	videoDeviceIndex := 0
	for _, line := range lines {
		if strings.Contains(line, "AVFoundation video devices:") {
			inVideoDevices = true
			continue
		}
		// Disregard the audio device
		if strings.Contains(line, "AVFoundation audio devices:") {
			inVideoDevices = false
			break
		}

		if inVideoDevices {

			// Format output
			trimmedLine := strings.TrimSpace(line)
			if strings.Contains(trimmedLine, "Capture screen 0") {
				fmt.Println("Located main device screen")
				return strconv.Itoa(videoDeviceIndex), nil
			}

			if strings.Contains(trimmedLine, "]") && len(trimmedLine) > 0 {
				videoDeviceIndex++
			}
		}
	}

	return "", errors.New("could not find 'Capture screen 0' in ffmpeg device list")
}
