package tracking

import (
	"context"
	"sync"
	"time"

	"github.com/go-vgo/robotgo"
	hook "github.com/robotn/gohook"
	"github.com/vedantwpatil/Screen-Capture/internal/logger"
)

// Captures the mouse position and times when the mouse is clicked
func StartMouseTracking(mouseEvents *[]CursorPosition, startingTime time.Time, targetFPS int, ctx context.Context) {
	// Guards mouseEvents: the poll loop below and the click hook run on
	// separate goroutines and both append to the same slice.
	var mu sync.Mutex

	// Register mouse location
	go func() {
		mousePos := CursorPosition{}
		for {
			select {

			case <-ctx.Done():
				logger.Info.Println("Mouse location tracking stopped")
				return
			default:
				xMouse, yMouse := robotgo.Location()

				currentTime := time.Now()
				elapsedTime := currentTime.Sub(startingTime)

				mousePos.X = int16(xMouse)
				mousePos.Y = int16(yMouse)

				mousePos.ClickTimeStamp = elapsedTime

				mu.Lock()
				*mouseEvents = append(*mouseEvents, mousePos)
				mu.Unlock()

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

			logger.Debug.Printf("Click detected at position (%d, %d) with timestamp: %v", e.X, e.Y, elapsedTime)

			clickEvent := CursorPosition{
				X:              e.X,
				Y:              e.Y,
				ClickTimeStamp: elapsedTime,
			}

			mu.Lock()
			*mouseEvents = append(*mouseEvents, clickEvent)
			mu.Unlock()
		}
	})

	evChan := hook.Start()

	logger.Info.Println("Hook process started, waiting for events")
	// Start processing events. This blocks until hook.End() is called.
	<-hook.Process(evChan)

	logger.Info.Println("Hook process stopped")
}
