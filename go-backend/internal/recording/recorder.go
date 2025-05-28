// internal/recording/recorder.go
package recording

import (
	"context"
	"errors"
	"fmt"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/vedantwpatil/Screen-Capture/internal/config"
	"github.com/vedantwpatil/Screen-Capture/internal/tracking"
)

type Recorder struct {
	config        *config.Config
	isRecording   bool
	isDone        bool
	outputPath    string
	cursorHistory []tracking.CursorPosition
	stopChan      chan struct{}
	doneChan      chan struct{}
	startTime     time.Time
	mu            sync.Mutex
}

func NewRecorder(config *config.Config) *Recorder {
	return &Recorder{
		config:   config,
		stopChan: make(chan struct{}),
		doneChan: make(chan struct{}),
	}
}

func (r *Recorder) Start(baseName string) error {
	r.mu.Lock()
	if r.isRecording {
		r.mu.Unlock()
		return fmt.Errorf("recording already in progress")
	}
	r.mu.Unlock()

	// Create output directory if it doesn't exist
	outputDir := "output"
	if err := os.MkdirAll(outputDir, 0755); err != nil {
		return fmt.Errorf("failed to create output directory: %w", err)
	}

	// Set up paths and state
	r.outputPath = filepath.Join(outputDir, baseName+".mp4")
	r.mu.Lock()
	r.isRecording = true
	r.isDone = false
	r.cursorHistory = make([]tracking.CursorPosition, 0)
	r.startTime = time.Now() // Set the start time
	r.mu.Unlock()

	// Create a context for mouse tracking
	ctx, cancel := context.WithCancel(context.Background())

	// Start recording in a goroutine
	go func() {
		r.startRecording()
		cancel() // Cancel the context when recording stops
	}()

	// Start mouse tracking in a goroutine
	go tracking.StartMouseTracking(
		&r.cursorHistory,
		r.startTime,
		r.config.Recording.TargetFPS,
		ctx,
	)

	return nil
}

func (r *Recorder) startRecording() {
	defer close(r.doneChan)

	var cmd *exec.Cmd
	osType := runtime.GOOS

	switch osType {
	case "darwin":
		index, err := findScreenDeviceIndex()
		if err != nil {
			log.Printf("Unable to capture the correct device screen: %v", err)
			return
		}
		cmd = exec.Command("ffmpeg",
			"-f", "avfoundation",
			"-framerate", fmt.Sprintf("%d", r.config.Recording.TargetFPS),
			"-i", index+":none",
			"-c:v", "libx264",
			"-pix_fmt", "yuv420p",
			"-preset", "ultrafast",
			"-y",
			r.outputPath)
	default:
		log.Printf("Unsupported operating system: %s", osType)
		return
	}

	stdinPipe, err := cmd.StdinPipe()
	if err != nil {
		log.Printf("Failed to get stdin pipe: %v", err)
		return
	}
	defer stdinPipe.Close()

	cmd.Stderr = os.Stderr

	if err := cmd.Start(); err != nil {
		log.Printf("Failed to start ffmpeg: %v", err)
		return
	}

	// Wait for stop signal
	go func() {
		<-r.stopChan
		stdinPipe.Write([]byte("q\n"))
		stdinPipe.Close()
	}()

	if err := cmd.Wait(); err != nil {
		log.Printf("FFmpeg process finished with status: %v", err)
	}

	r.mu.Lock()
	r.isRecording = false
	r.isDone = true
	r.mu.Unlock()
}

func (r *Recorder) Stop() error {
	r.mu.Lock()
	if !r.isRecording {
		r.mu.Unlock()
		return fmt.Errorf("no recording in progress")
	}
	r.mu.Unlock()

	// Signal recording to stop
	close(r.stopChan)

	// Wait for recording to finish
	<-r.doneChan

	// Reset channels for next recording
	r.stopChan = make(chan struct{})
	r.doneChan = make(chan struct{})

	return nil
}

func (r *Recorder) IsRecording() bool {
	r.mu.Lock()
	defer r.mu.Unlock()
	return r.isRecording
}

func (r *Recorder) IsDone() bool {
	r.mu.Lock()
	defer r.mu.Unlock()
	return r.isDone
}

func (r *Recorder) GetOutputPath() string {
	return r.outputPath
}

func (r *Recorder) GetCursorHistory() []tracking.CursorPosition {
	return r.cursorHistory
}

func findScreenDeviceIndex() (string, error) {
	cmd := exec.Command("ffmpeg", "-f", "avfoundation", "-list_devices", "true", "-i", "")

	outputBytes, err := cmd.CombinedOutput()
	if err != nil {
		if len(outputBytes) == 0 {
			return "", fmt.Errorf("failed to run ffmpeg list_devices command: %v, output: %s", err, outputBytes)
		}

		fmt.Println("Ffmpeg list_devices exited non-zero, but produced output. Proceeding with parsing.")
	}

	output := string(outputBytes)
	lines := strings.Split(output, "\n")

	// Get main desktop device index
	inVideoDevices := false
	videoDeviceIndex := 0
	for _, line := range lines {
		if strings.Contains(line, "AVFoundation video devices:") {
			inVideoDevices = true
			continue
		}
		// TODO: Add audio support
		// Currently not capturing the audio
		if strings.Contains(line, "AVFoundation audio devices:") {
			inVideoDevices = false
			break
		}

		if inVideoDevices {

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

func GetVideoResolution(path string) (string, error) {
	cmd := exec.Command("ffprobe", "-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height", "-of", "csv=s=x:p=0", path)
	out, err := cmd.CombinedOutput()
	if err != nil {
		return "Failed to get the video resolution. The file path tried was: " + path, err
	}
	resolution := strings.TrimSpace(string(out))
	return resolution, nil
}
