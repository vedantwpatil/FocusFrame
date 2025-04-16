package editing

import (
	"fmt"

	vidio "github.com/AlexEidt/Vidio"
)

func EditVideoFile(filePath string) {
}

func processVideoFile(filePath string) error {
	vid, err := vidio.NewVideo(filePath)
	if err != nil {
		fmt.Errorf("Unable to process video file", err)
	}
	defer vid.Close()
	for vid.Read() {
		frameBytes := vid.FrameBuffer()
	}

	fmt.Println("Finished processing video stream")
	return nil
}
