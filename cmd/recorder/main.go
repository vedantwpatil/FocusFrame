package main

import (
	"context"
	"encoding/csv"
	"fmt"
	"log"
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
		// File and video variables
		targetFPS            = 60
		isRecording          = false
		recordMutex          = &sync.Mutex{}
		stopChan             = make(chan struct{})
		outputFilePath       string
		editedOutputFilePath string
		baseName             string

		// Time tracking
		timeStarted   time.Time
		cursorHistory []tracking.CursorPosition
		recordingDone = make(chan struct{})

		// Csv writing
		file   *os.File
		writer *csv.Writer
	)
	ctx, cancel := context.WithCancel(context.Background())

	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)

	// CSV file for mouse positions
	file, err := os.Create("output/data.csv")
	if err != nil {
		log.Fatalf("Failed creating file: %s", err)
	}
	defer file.Close()

	writer = csv.NewWriter(file)
	defer writer.Flush()

	header := []string{"Event", "X", "Y", "Timestamp"}
	if err := writer.Write(header); err != nil {
		log.Fatalf("Error writing header : %s", err)
	}

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
				cancel() // Cancel the context
				return   // Exit the goroutine
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

			// Outputs the pre edited video filepath for debugging
			outputFilePath = "output/" + baseName + ".mp4"
			editedOutputFilePath = "output/" + baseName + "-edited.mp4"
			fmt.Printf("Output file: %s\n", outputFilePath)

			fmt.Println("Starting screen recording... Press Ctrl+C to stop recording.")
			go recording.StartRecording(outputFilePath, stopChan, recordingDone, targetFPS)
			timeStarted = time.Now()

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
			fmt.Println("Exiting application...")
			// Ensure everything is cleaned up before exiting

			recordMutex.Lock()
			if isRecording {
				close(stopChan)
				cancel()
			}
			recordMutex.Unlock()

			// Convert cursorHistory data to [][]string format
			records := [][]string{}
			for i, pos := range cursorHistory {
				// Calculate the absolute time of the click event
				actualClickEventTime := timeStarted.Add(pos.ClickTimeStamp)

				record := []string{
					fmt.Sprintf("%d", i),
					fmt.Sprintf("%d", pos.X),
					fmt.Sprintf("%d", pos.Y),
					actualClickEventTime.Format(time.RFC3339),
				}
				records = append(records, record)
			}

			// Write all records at once
			if err := writer.WriteAll(records); err != nil {
				log.Fatalf("Error writing records: %s", err)
			}

			// TODO: Test if it accurately writes to csv files
			fmt.Printf("Mouse tracking data saved to data.csv (%d records)\n", len(records))
			fmt.Println("Exiting application...")
			os.Exit(0)

		default:
			fmt.Println("Invalid option")
		}
	}
}
