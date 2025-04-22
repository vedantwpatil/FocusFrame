package tracking

import (
	"context"
	"fmt"
	"time"

	"github.com/go-vgo/robotgo"
	hook "github.com/robotn/gohook"
)

// Captures the mouse position and times when the mouse is clicked
func StartMouseTracking(x *[]int16, y *[]int16, timesClicked *[]time.Duration, startingTime time.Time, ctx context.Context) {
	// Register location
	go func() {
		for {
			select {

			case <-ctx.Done():
				fmt.Println("Mouse location tracking stopped...")
				return
			default:
				xMouse, yMouse := robotgo.Location()

				*x = append(*x, int16(xMouse))
				*y = append(*y, int16(yMouse))
				// To avoid high/wasted cpu usage
				time.Sleep(10 * time.Millisecond)
			}
		}
	}()

	// Register click times
	hook.Register(hook.MouseDown, []string{}, func(e hook.Event) {
		if e.Button == hook.MouseMap["left"] || e.Button == 1 {
			currentTime := time.Now()
			elapsedTime := currentTime.Sub(startingTime)

			*timesClicked = append(*timesClicked, elapsedTime)

		}
	})

	// Start the event hook listener
	evChan := hook.Start()

	fmt.Println("Hook process started. Waiting for events...")
	// Start processing events. This blocks until hook.End() is called.
	<-hook.Process(evChan)

	fmt.Println("Hook process stopped.")
}
