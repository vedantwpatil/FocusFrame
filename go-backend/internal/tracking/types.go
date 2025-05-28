package tracking

import "time"

// MouseEvent holds information about a mouse click event during recording.
// Exported fields (starting with uppercase) allow access from other packages.
type CursorPosition struct {
	X              int16         // X coordinate of the mouse click
	Y              int16         // Y coordinate of the mouse click
	ClickTimeStamp time.Duration // Time elapsed since recording started
	Velocity       float64
}

// You might also define a slice type for convenience if needed elsewhere:
// type MouseEvents []MouseEvent
