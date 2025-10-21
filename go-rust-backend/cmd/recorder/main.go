package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"os/signal"
	"syscall"

	"github.com/vedantwpatil/Screen-Capture/internal/config"
	"github.com/vedantwpatil/Screen-Capture/internal/editing"
	"github.com/vedantwpatil/Screen-Capture/internal/recording"
)

type Application struct {
	config   *config.Config
	recorder *recording.Recorder
	ctx      context.Context
	cancel   context.CancelFunc
}

func NewApplication() *Application {
	ctx, cancel := context.WithCancel(context.Background())
	return &Application{
		config: config.NewConfig(),
		ctx:    ctx,
		cancel: cancel,
	}
}

func (app *Application) Run() error {
	// Set up signal handling
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)

	// Handle signals
	go app.handleSignals(sigChan)

	// Main application loop
	for {
		if err := app.showMenu(); err != nil {
			return err
		}
	}
}

func (app *Application) showMenu() error {
	fmt.Println("\nCommands:")
	fmt.Println("1. Start recording")
	fmt.Println("2. Edit video after recording")
	fmt.Println("3. Exit")
	fmt.Print("Choose an option: ")

	var choice int
	if _, err := fmt.Scanln(&choice); err != nil {
		return fmt.Errorf("invalid input: %w", err)
	}

	switch choice {
	case 1:
		return app.startRecording()
	case 2:
		return app.editVideo()
	case 3:
		return app.cleanup()
	default:
		fmt.Println("Invalid option")
		return nil
	}
}

func (app *Application) startRecording() error {
	if app.recorder != nil && app.recorder.IsRecording() {
		fmt.Println("Already recording")
		return nil
	}

	baseName, err := app.getBaseName()
	if err != nil {
		return err
	}

	app.recorder = recording.NewRecorder(app.config)
	return app.recorder.Start(baseName)
}

func (app *Application) getBaseName() (string, error) {
	fmt.Print("Enter the name you wish to save the file under (Don't include the file format ex .mp4): ")
	var baseName string
	if _, err := fmt.Scanln(&baseName); err != nil {
		return "", fmt.Errorf("failed to read base name: %w", err)
	}
	return baseName, nil
}

func (app *Application) editVideo() error {
	if app.recorder == nil || !app.recorder.IsDone() {
		fmt.Println("No completed recording available for editing")
		return nil
	}

	fmt.Println("\nStarting video processing...")

	inputPath := app.recorder.GetOutputPath()
	outputPath := inputPath[:len(inputPath)-4] + "-edited.mp4"
	mouseHistory := app.recorder.GetCursorHistory()

	fmt.Printf("Input: %s\n", inputPath)
	fmt.Printf("Output: %s\n", outputPath)
	fmt.Printf("Mouse events captured: %d\n", len(mouseHistory))

	// Check if we have enough mouse data
	if len(mouseHistory) < 4 {
		return fmt.Errorf("not enough mouse data for smoothing (need at least 4 points, got %d)", len(mouseHistory))
	}

	// Process the video
	err := editing.ProcessEffect(
		inputPath,
		outputPath,
		mouseHistory,
		int16(app.config.Recording.TargetFPS),
	)
	if err != nil {
		return fmt.Errorf("video processing failed: %w", err)
	}

	fmt.Println("\n✨ Video processing complete!")
	fmt.Printf("📁 Edited video saved to: %s\n", outputPath)

	return nil
}

func (app *Application) cleanup() error {
	if app.recorder != nil {
		if err := app.recorder.Stop(); err != nil {
			return err
		}
	}
	app.cancel()
	return nil
}

func (app *Application) handleSignals(sigChan chan os.Signal) {
	for sig := range sigChan {
		fmt.Printf("\nReceived signal: %v\n", sig)
		if app.recorder != nil && app.recorder.IsRecording() {
			fmt.Println("Stopping recording...")
			if err := app.recorder.Stop(); err != nil {
				log.Printf("Error stopping recording: %v", err)
			}
		} else {
			fmt.Println("Exiting application...")
			app.cancel()
			return
		}
	}
}

func main() {
	app := NewApplication()
	if err := app.Run(); err != nil {
		log.Fatalf("Application error: %v", err)
	}
}
