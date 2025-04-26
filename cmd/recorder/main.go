package main

import (
	"context"
	"fmt"
	"os"
	"os/signal"
	"sync"
	"syscall"
	"time"

	hook "github.com/robotn/gohook"
	"github.com/vedantwpatil/Screen-Capture/internal/editing"
	"github.com/vedantwpatil/Screen-Capture/internal/recording"
	"github.com/vedantwpatil/Screen-Capture/internal/tracking"
)

// TODO: Need to manage channels using context instead of sending signals
func main() {
	// Recording state variables
	var (
		targetFPS            = 60
		isRecording          = false
		recordMutex          = &sync.Mutex{}
		stopChan             = make(chan struct{})
		outputFilePath       string
		editedOutputFilePath string
		baseName             string

		cursorHistory []tracking.CursorPosition
		recordingDone = make(chan struct{})
	)
	ctx, cancel := context.WithCancel(context.Background())

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
				recordMutex.Unlock()

				continue
			} else {
				// If we're not recording then we should stop the program
				fmt.Println("Exiting application...")
				recordMutex.Unlock()
				os.Exit(0)
			}
		}
	}()
	for {
		fmt.Println("\nCommands:")
		fmt.Println("1. Start recording")
		fmt.Println("2. Edit video after recording")
		fmt.Println("3. Exit")
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

			recordingDone = make(chan struct{})
			stopChan = make(chan struct{})
			isRecording = true
			recordMutex.Unlock()

			// Save file name
			fmt.Print("Enter the name you wish to save the file under (Don't include the file format ex .mp4): ")
			fmt.Scanln(&baseName)
			outputFilePath = baseName + ".mp4"
			fmt.Printf("Output file: %s\n", outputFilePath)

			fmt.Println("Starting screen recording... Press Ctrl+C to stop recording.")
			go recording.StartRecording(outputFilePath, stopChan, recordingDone, targetFPS)
			timeStarted := time.Now()

			fmt.Println("Starting mouse tracking...")
			go tracking.StartMouseTracking(&cursorHistory, timeStarted, targetFPS, ctx)

		case 2:
			// Wait for recording to be done
			<-recordingDone
			// End mouse tracking
			hook.End()

			fmt.Println("Starting video editing...")
			editing.EditVideoFile(outputFilePath, editedOutputFilePath, cursorHistory, float64(targetFPS))
			fmt.Println("Video editing complete.")

		case 3:
			recordMutex.Lock()
			if isRecording {
				close(stopChan)
				cancel()
			}
			recordMutex.Unlock()
			fmt.Println("Exiting application...")

			// Print mouse locations for debugging
			fmt.Println("Cursor history details:")
			for i, pos := range cursorHistory {
				fmt.Printf("  Event %d: X=%d, Y=%d, Timestamp=%v\n", i, pos.X, pos.Y, pos.ClickTimeStamp)
			}

			os.Exit(0)

		default:
			fmt.Println("Invalid option")
		}
	}
}
