# 2. Cursor timeline leads the video

**Severity:** High
**Phase:** recording → rendering
**Status:** confirmed by source reading; magnitude has a measured floor

This bug is **independent** of the coordinate-space defect and survives fixing it.

---

## Symptom

The rendered cursor arrives at buttons, links and text fields *before* the
recorded UI reacts to being clicked. Everything appears shifted early by a few
hundred milliseconds.

## Cause

Two clocks that are assumed to be the same clock, but are not.

### Clock A — the cursor clock starts too early

`internal/recording/recorder.go:57-79`:

```go
r.mu.Lock()
r.isRecording = true
r.isDone = false
r.cursorHistory = make([]tracking.CursorPosition, 0)
r.startTime = time.Now()          // <-- line 61
r.mu.Unlock()

ctx, cancel := context.WithCancel(context.Background())

go func() {
    r.startRecording()            // ffmpeg has not been launched yet
    cancel()
}()

go tracking.StartMouseTracking(&r.cursorHistory, r.startTime, ...)
```

`startTime` is stamped **before** the goroutine that launches `ffmpeg` even runs.
`startRecording()` then does, in order:

1. `findScreenDeviceIndex()` — shells out `ffmpeg -f avfoundation -list_devices true`.
   **Measured: 0.28 s.**
2. `exec.Command(...)` + `cmd.Start()` — process spawn.
3. AVFoundation capture-session negotiation before the first frame is delivered.

Mouse polling, meanwhile, begins immediately and timestamps everything relative to
`startTime`.

### Clock B — the renderer assumes cursor t=0 ≡ video frame 0

On the Rust side:

- `video.rs` `build_cursor_lookup` rebases the cursor timeline so t=0 is the
  **first cursor sample**:
  ```rust
  let start_time = cursor_points[0].timestamp_ms;
  cursor_points.iter().map(|p| (p.timestamp_ms - start_time, p.x, p.y)).collect()
  ```
- `video.rs` `process_single_frame` derives cursor time purely from the frame
  index:
  ```rust
  let timestamp_ms = frame_count as f64 * time_base_seconds * 1000.0;
  let (cx, cy) = interpolate_cursor_position(cursor_lookup, timestamp_ms);
  ```

So video frame 0 is aligned to the *first mouse sample*, which was taken before
`ffmpeg` existed.

### Net effect

All pointer motion recorded during device enumeration, process spawn and capture
session setup is glued onto the front of the video. Everything after it is shifted
early by that same amount.

---

## Magnitude

| Component | Value |
|---|---|
| `findScreenDeviceIndex()` | **0.28 s** (measured) |
| `cmd.Start()` process spawn | not isolated |
| AVFoundation session open → first frame | **not cleanly measured** |
| **Total lead** | **≥ 0.28 s** |

0.28 s is a **floor**, not the answer. The AVFoundation session-open latency is
the dominant unknown and is likely to be of comparable or larger size. Measuring
it requires capturing the screen, which was not re-run.

---

## Fix direction

Do not stamp a wall clock three function calls before the recorder exists. Either:

- stamp `startTime` at **first frame delivered** (requires parsing ffmpeg's
  progress output or switching to a piped capture), or
- carry a real PTS alignment between the two streams rather than inferring it
  from a Go `time.Now()`.

Moving `findScreenDeviceIndex()` earlier (before `startTime` is set) removes the
one measured component cheaply, but leaves the session-open lead intact. It is a
mitigation, not a fix.
