package main

import (
	"fmt"
	"image/png"
	"os"

	"github.com/kbinani/screenshot"
)

func main() {
	// Get number of active displays
	n := screenshot.NumActiveDisplays()

	for i := range n {
		// Get display boundaries
		bounds := screenshot.GetDisplayBounds(i)

		// Capture the screen area
		img, err := screenshot.CaptureRect(bounds)
		if err != nil {
			panic(err)
		}

		fileName := fmt.Sprintf("%d_%dx%d.png", i, bounds.Dx(), bounds.Dy())
		file, _ := os.Create(fileName)
		defer file.Close()

		png.Encode(file, img)

		fmt.Printf("Display #%d : %v \"%s\"\n", i, bounds, fileName)
	}
}
