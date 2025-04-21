package tracking

import (
	"fmt"
	"time"

	hook "github.com/robotn/gohook"
)

// Captures the mouse position and times when the mouse is clicked
func StartMouseTracking(x *[]int16, y *[]int16, timesClicked *[]time.Duration, startingTime time.Time) {
	// TODO: Verify that this function works
	hook.Register(hook.MouseDown, []string{}, func(e hook.Event) {
		if e.Button == hook.MouseMap["left"] || e.Button == 1 {
			currentTime := time.Now()
			elapsedTime := currentTime.Sub(startingTime)

			// Dereference the pointers (*) to access the original slices
			// and assign the result of append back to the dereferenced pointer.
			*x = append(*x, e.X)
			*y = append(*y, e.Y)
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
