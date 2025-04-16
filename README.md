# Focus Frame

This program captures screen recording and will be able to add high quality smooth zoom animations on click as well as record high quality video.

## Project Timeline

Able to take screen recordings through taking multiple screenshots per second and then encoding them all together using ffmpeg. Was unable to capture more than (First iteration) 5FPS (Second Iteration) 7 FPS and looking at alternative methods to increase frame rate utilizing the screenshots package.

Now we've switched over to using ffmpeg to capture the screen and encode the captured video and getting higher frame rate.

Optimized screen recording in ffmpeg to automatically detect and record primary screen on macos.

Need to restructure the project since all the logic is currently in the main.go file

### Current Status

Working on finding a way to track the mouse to be able to add high quality zoom animations when the mouse is clicked.

### Potential Features

- Adding audio to the screen recording
- Adding webcam for camera feedback
- Adding GUI for user to choose screens and capture area
- Adding editing interface for user to edit the captured video and audio
