# Focus Frame

This program captures screen recording and will be able to add high quality smooth zoom animations on click as well as record high quality video. I began writing this in go to get a better handle on the more advanced features of go. Python was another consideration for this project however the performance of the video encoding was a major consideration and was thought to be a lot slower so I avoided that. Finally I found out later that being able to create one single executable in go would ensure high usability on multiple platforms and that was an additional nice touch.

## Project Timeline

Able to take screen recordings through taking multiple screenshots per second and then encoding them all together using ffmpeg. Was unable to capture more than (First iteration) 5FPS (Second Iteration) 7 FPS and looking at alternative methods to increase frame rate utilizing the screenshots package.

Now we've switched over to using ffmpeg to capture the screen and encode the captured video and getting higher frame rate.

Optimized screen recording in ffmpeg to automatically detect and record primary screen on macos utilizing goroutines for async screen capture and video render for faster render times.

### Current Status

Working on finding a way to track the mouse to be able to add the video effect animations when the mouse is clicked. Currently utilizing Cgo to integrate some rust code for the mouse smoothening algorithm while utilizing go with ffmpeg for other effects like the zoom and blurring. I've also added the front-end to know what to build later on and the architecture that I've decided on. The primary focus is first creating the mouse smoothening algorithm and from there to then add the mouse tracking engine. 

Something for consideration is doing all of the video effects processing in rust and then just calling it from go although I am not sure about this but it feels like it would work/make sense. 

## Potential Features

- Adding cursor hiding for when the cursor is static
- Adding audio to the screen recording
- Adding webcam for camera feedback
- Adding GUI for user to choose screens and capture area
- Adding editing interface for user to edit the captured video and audio

## Hardware

The software was written on macOS utilizing the m3 max chip so the frame rate and video encoding speeds may vary

## Dependencies

This software requires golang as well as ffprobe and ffmpeg which it uses for the primary screen capture and the video editing pipeline
