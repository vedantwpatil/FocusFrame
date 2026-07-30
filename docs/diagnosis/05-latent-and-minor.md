# 5. Latent and minor issues

**Severity:** Low
None of these explain the current wrong output, but all are real.

---

## 5a. Frame stride is ignored (latent — not firing today)

`video.rs:323-329`, in `overlay_cursor_on_frame`:

```rust
// IMPORTANT: Handle frame stride (pitch)
// composite_cursor_subpixel must iterate rows using stride, not just width*4
let _stride = frame.stride(0); // TODO: Pass stride to renderer for non-contiguous frames
let data = frame.data_mut(0);
// Call renderer (Update your renderer.rs to accept stride!)
// If renderer.rs is not updated, this assumes stride == width * 4 (Risky but common)
composite_cursor_subpixel(data, width, height, cursor_sprite, x, y);
```

The code comments already identify this correctly; it is a known-open TODO.

`renderer.rs` then assumes `stride == width * 4` throughout:

```rust
let draw_end_y = end_y.min((frame.len() / (frame_width as usize * 4)) as i32);
...
let idx = ((dy as u32 * frame_width + dx as u32) * 4) as usize;
```

FFmpeg pads `linesize` to an alignment boundary, so `stride` is only guaranteed
equal to `width * 4` when `width * 4` already meets that alignment.

Current widths happen to satisfy it:

| Width | `width * 4` | 64-byte aligned? |
|---|---|---|
| 5120 | 20480 | yes |
| 3024 | 12096 | yes |

So this is **dormant**. Any capture width not a multiple of 16 would produce a
cursor sheared diagonally across the frame — a dramatic and confusing symptom
whose cause would not be obvious. Worth closing while the code is being touched
anyway.

---

## 5b. Recordings under one second get timestamps multiplied by 1000

`smoothing.rs:318-327`, in `normalize_to_relative_ms` (defined at line 297):

```rust
// HEURISTIC: If relative duration is small (< 1000), it's definitely Seconds.
// (A 1000ms video is 1 second, unlikely to be the full recording).
// Screen recordings are typically 5s - 300s.
if duration > 0.0 && duration < 1000.0 {
    log::info!("Detected SECONDS (Duration: {:.2}s). Converting to MS.", duration);
    for p in &mut relative_points {
        p.timestamp_ms *= 1000.0;
    }
}
```

The intent is to auto-detect timestamps supplied in seconds rather than
milliseconds. But a legitimate recording shorter than one second has a genuine
duration below 1000 ms and is silently scaled by 1000 — stretching the cursor
timeline to ~1000× the video length, so the cursor barely moves.

The Go side unconditionally sends milliseconds (`effects.go:123`):

```go
timestampMillis := float64(p.ClickTimeStamp.Nanoseconds()) / 1_000_000.0
```

There is no caller that supplies seconds. The heuristic has no legitimate case
and should be deleted rather than have its threshold tuned.

---

## 5c. Progress percentage is computed against the wrong duration

`video.rs:145`:

```rust
let estimated_total_frames = ((end_ts - start_ts) / 1000.0 * config.frame_rate as f64) as u64;
```

`start_ts` / `end_ts` come from the **cursor** track, not the video. Given
[finding 2](02-timing-offset.md) these two durations always differ, so the
reported percentage is always somewhat wrong and can exceed 100%.

Cosmetic — affects the progress bar only, not the output.

---

## 5d. Cursor sprite path is CWD-relative

`internal/video/pipeline.go`:

```go
cursorSpritePath := "internal/video/cursor-sprite.png"
```

This resolves against the process working directory, so the binary only works
when run from the module root. Running it from anywhere else fails to load the
sprite.

Should be resolved relative to the executable, or embedded with `go:embed`.

---

## 5e. Note on error visibility

The Rust FFI boundary wraps work in `catch_unwind(AssertUnwindSafe(...))` and maps
panics to `ERR_RENDERING_FAILED (-4)`. That is correct defensive design, but it
means a panic surfaces to the user only as a numeric code. Given how many of the
above are silent-degradation failures rather than hard errors, richer error
reporting across the boundary would shorten future diagnosis considerably.
