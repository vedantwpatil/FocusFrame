package main

import (
	"fmt"

	hook "github.com/robotn/gohook"
	// robotgo might still be needed for GetMousePos if event doesn't provide coords
	// "github.com/go-vgo/robotgo"
)

func listenClick() {
	fmt.Println("Starting event listener ")
	fmt.Println("Press ctrl + shift + q to stop")

	// Register a listener for left mouse down events
	hook.Register(hook.MouseDown, []string{}, func(e hook.Event) {
		// Check if it's the left button (Button 1 is typically left)
		if e.Button == hook.MouseMap["left"] {
			// Get mouse position
			fmt.Printf("Left Mouse Down detected at: X=%d, Y=%d\n", e.X, e.Y)
		}
	})

	// Register a combination to stop the listener
	hook.Register(hook.KeyDown, []string{"q", "ctrl", "shift"}, func(e hook.Event) {
		fmt.Println("ctrl-shift-q detected. Stopping listener.")
		hook.End()
	})

	// Start the event hook
	s := hook.Start()

	// Wait until the listener is stopped
	<-hook.Process(s)

	fmt.Println("--- Event listener stopped ---")
}
