package main

import (
	"fmt"
	"os"
	"os/signal"
	"sync"
	"syscall"
	"time"

	hook "github.com/robotn/gohook"
	"github.com/vedantwpatil/Screen-Capture/internal/recording"
	"github.com/vedantwpatil/Screen-Capture/internal/tracking"
)

func main() {
	// Recording state variables
	var (
		isRecording = false
		recordMutex = &sync.Mutex{}
		stopChan    = make(chan struct{})

		mouseLocationsX []int16
		mouseLocationsY []int16
		timeSpots       []time.Duration
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
			go recording.StartRecording(stopChan)
			timeStarted := time.Now()

			fmt.Println("Starting mouse tracking...")
			go tracking.StartMouseTracking(&mouseLocationsX, &mouseLocationsY, &timeSpots, timeStarted)

		case 2:
			recordMutex.Lock()
			if isRecording {
				// Close the channel and stop the mouse tracking
				hook.End()
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
