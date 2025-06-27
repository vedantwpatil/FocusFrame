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

// Need to combine the methods smooth cusor path and catmull rom spline.
// Test the catmull rom spline generation algorithm

func CatmullRomSpline(point_0, point_1, point_2, point_3 tracking.CursorPosition, numPoints int, alpha float64) []tracking.CursorPosition {
	var knot_0 float64 = 0

	knot_1 := CalculateKnots(knot_0, alpha, point_0, point_1)
	knot_2 := CalculateKnots(knot_1, alpha, point_1, point_2)
	knot_3 := CalculateKnots(knot_2, alpha, point_2, point_3)

	// Finish implementing catmull rom spline cursor path geneartion
	t := Reshape(Linspace(knot_1, knot_2, numPoints))

	tValues := Linspace(knot_1, knot_2, numPoints)

	splinePoints := make([]tracking.CursorPosition, numPoints)

	for i, t := range tValues {

		// A1
		coeffA1_P0 := (knot_1 - t) / (knot_1 - knot_0)
		coeffA1_P1 := (t - knot_0) / (knot_1 - knot_0)
		A1 := point_0.Scale(coeffA1_P0).Add(point_1.Scale(coeffA1_P1))

		// A2
		coeffA2_P1 := (knot_2 - t) / (knot_2 - knot_1)
		coeffA2_P2 := (t - knot_1) / (knot_2 - knot_1)
		A2 := point_1.Scale(coeffA2_P1).Add(point_2.Scale(coeffA2_P2))

		// A3
		coeffA3_P2 := (knot_3 - t) / (knot_3 - knot_2)
		coeffA3_P3 := (t - knot_2) / (knot_3 - knot_2)
		A3 := point_2.Scale(coeffA3_P2).Add(point_3.Scale(coeffA3_P3))

		// B1
		coeffB1_A1 := (knot_2 - t) / (knot_2 - knot_0)
		coeffB1_A2 := (t - knot_0) / (knot_2 - knot_0)
		B1 := A1.Scale(coeffB1_A1).Add(A2.Scale(coeffB1_A2))

		// B2
		coeffB2_A2 := (knot_3 - t) / (knot_3 - knot_1)
		coeffB2_A3 := (t - knot_1) / (knot_3 - knot_1)
		B2 := A2.Scale(coeffB2_A2).Add(A3.Scale(coeffB2_A3))

		// Final point (P)
		coeffP_B1 := (knot_2 - t) / (knot_2 - knot_1)
		coeffP_B2 := (t - knot_1) / (knot_2 - knot_1)
		splinePoints[i] = B1.Scale(coeffP_B1).Add(B2.Scale(coeffP_B2))
	}
	return splinePoints
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

// Creates a line of equally spaced out numbers
func Linspace(start, stop float64, numPoints int) []float64 {
	if numPoints <= 1 {
		return []float64{start}
	}

	values := make([]float64, numPoints)
	step := (stop - start) / (float64(numPoints) - 1)

	for i := 0; i < numPoints; i++ {
		values[i] = start + float64(i)*step
	}
	return values
}

// Translates a array of numbers from row to column form
func Reshape(data []float64) [][]float64 {
	result := make([][]float64, len(data))
	for i, val := range data {
		result[i] = []float64{val}
	}
	return result
}
