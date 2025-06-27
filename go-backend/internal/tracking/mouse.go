package tracking

import (
	"context"
	"fmt"
	"time"

	"github.com/go-vgo/robotgo"
	hook "github.com/robotn/gohook"
)

// Captures the mouse position and times when the mouse is clicked
func StartMouseTracking(mouseEvents *[]CursorPosition, startingTime time.Time, targetFPS int, ctx context.Context) {
	// Register mouse location
	go func() {
		mousePos := CursorPosition{}
		for {
			select {

			case <-ctx.Done():
				fmt.Println("Mouse location tracking stopped...")
				return
			default:
				xMouse, yMouse := robotgo.Location()

				mousePos.X = int16(xMouse)
				mousePos.Y = int16(yMouse)
				mousePos.ClickTimeStamp = -1

				*mouseEvents = append(*mouseEvents, mousePos)
				// To capture mouse location only at every frame
				time.Sleep(1 * time.Second / time.Duration(targetFPS))
			}
		}
	}()

	// Register mouse click times
	hook.Register(hook.MouseDown, []string{}, func(e hook.Event) {
		if e.Button == hook.MouseMap["left"] || e.Button == 1 {

			currentTime := time.Now()
			elapsedTime := currentTime.Sub(startingTime)

			// Log click events
			fmt.Printf("Click detected at position (%d, %d) with timestamp: %v\n", e.X, e.Y, elapsedTime)

			clickEvent := CursorPosition{
				X:              e.X,
				Y:              e.Y,
				ClickTimeStamp: elapsedTime,
			}
			*mouseEvents = append(*mouseEvents, clickEvent)
		}
	})

	evChan := hook.Start()

	fmt.Println("Hook process started. Waiting for events...")
	// Start processing events. This blocks until hook.End() is called.
	<-hook.Process(evChan)

	fmt.Println("Hook process stopped.")
}

// Scale scales a CursorPosition by a scalar.
func (p CursorPosition) Scale(s float64) CursorPosition {
	return CursorPosition{X: p.X * int16(s), Y: p.Y * int16(s), ClickTimeStamp: p.ClickTimeStamp}
}

// Add adds two CursorPositions.
func (p1 CursorPosition) Add(p2 CursorPosition) CursorPosition {
	return CursorPosition{X: p1.X + p2.X, Y: p1.Y + p2.Y, ClickTimeStamp: p1.ClickTimeStamp}
}

// Subtract subtracts p2 from p1.
func (p1 CursorPosition) Subtract(p2 CursorPosition) CursorPosition {
	return CursorPosition{X: p1.X - p2.X, Y: p1.Y - p2.Y, ClickTimeStamp: p1.ClickTimeStamp}
}
