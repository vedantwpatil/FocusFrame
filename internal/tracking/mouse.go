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
