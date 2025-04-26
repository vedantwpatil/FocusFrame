package editing

import (
	"log"

	vidio "github.com/AlexEidt/Vidio"
	"github.com/vedantwpatil/Screen-Capture/internal/tracking"
)

// Takes in a video file with information about the mouse click locations and click times and edits the video with cinnematic transitions on the mouse
func EditVideoFile(inputFilePath, outputFilePath string, cursorHistory []tracking.CursorPosition, targetFPS float64) {
	video, err := vidio.NewVideo(inputFilePath)
	if err != nil {
		log.Fatalf("Unable to open the screen recorded video at path: %s \n ERROR: %v", inputFilePath, err)
	}

	options := vidio.Options{
		FPS:     video.FPS(),
		Bitrate: video.Bitrate(),
	}

	writer, err := vidio.NewVideoWriter(inputFilePath, video.Width(), video.Height(), &options)
	if err != nil {
		log.Fatalf("Unable to initialize video editor \n ERROR: %v ", err)
	}

	defer writer.Close()

	// For each frame in the video
	for video.Read() {
		// Smooth Cursor movement
		//

		//
		// Blur in the beginning of the zoom
		//
		//
		// Zoom in on mouse click (Ensure it follows laws of physics)
		//

		//
		// Mouse tracking engine (follows the mouse for a few seconds)

		//
		// Zooms out (Ensure it follows laws of physics)
		//

		//
		// Blur in the end of the zoom
		//

		//
		// Export the edited video into the different file
	}
}

func applyBlurAffect(video vidio.Video, mouseClickTimes []int16) {
	// frame := video.FrameBuffer()

	// go to the click time
}
