package video

import (
	"math"

	"github.com/vedantwpatil/Screen-Capture/internal/tracking"
)

func SmoothCursorPath(rawPoints []tracking.CursorPosition, alpha, tension, friction, mass float64, frameRate int16) {
}

func GenerateSmoothenedCursorPath(rawPoints []tracking.CursorPosition, alpha, tension, friction, mass float64, frameRate int16) []tracking.CursorPosition {
	quadrupleSize := 4
}

func Num_segments(pointChain []tracking.CursorPosition, quadrupleSize int) int {
	return len(pointChain) - (quadrupleSize - 1)
}

func CatmullRomSpline(point_0 tracking.CursorPosition, point_1 tracking.CursorPosition, point_2 tracking.CursorPosition, point_3 tracking.CursorPosition) {
	var knot_0 float64 = 0

	knot_1 := CalculateKnots(knot_0, 0.5, point_0, point_1)
	knot_2 := CalculateKnots(knot_1, 0.5, point_1, point_2)
	knot_3 := CalculateKnots(knot_2, 0.5, point_2, point_3)

	// Finish implementing catmull rom spline cursor path geneartion
}

func CalculateKnots(knot_1 float64, alpha float64, point_1, point_2 tracking.CursorPosition) float64 {
	p1x := point_1.X
	p1y := point_1.Y

	p2x := point_2.X
	p2y := point_2.Y

	dy := p2y - p1y
	dx := p2x - p1x

	l := (math.Pow(float64(dx), 2) + math.Pow(float64(dy), 2))

	return knot_1 + math.Pow(l, alpha)
}

func CalculateFramesInBetweenClicks(cursorHistory []tracking.CursorPosition, frameRate int16) []int64 {
	var numFrames []int64

	for i := range len(cursorHistory) - 1 {
		clickTime := cursorHistory[i].ClickTimeStamp
		nextClickTime := cursorHistory[i+1].ClickTimeStamp

		amtTime := nextClickTime - clickTime
		amtFrames := frameRate * int16(amtTime)
		numFrames = append(numFrames, int64(amtFrames))
	}
	return numFrames
}
