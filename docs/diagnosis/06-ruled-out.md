# 6. Ruled out

Hypotheses that were actively tested and eliminated. Recorded so they are not
re-investigated.

---

## Capture quality / dropped frames — **clean**

`testing.mp4` (2026-07-30):

| Metric | Value |
|---|---|
| Frames | 1243 |
| Duration | 20.733 s |
| Median PTS delta | 16.67 ms (exactly 1/60 s) |
| Frames with delta > 25 ms | **2** |

AVFoundation capture is delivering essentially perfect 60 fps. The recording
phase is not the problem.

---

## VFR → CFR conversion — **correct**

The filter graph in `video.rs` is built manually as:

```
buffer → fps=60:round=near → format=pix_fmts=rgba → buffersink
```

Verified end to end: **1243 input frames → 1244 output frames**, and
`20.733 s × 60 = 1244`. The `fps` filter and the frame-index-based PTS assignment
are both behaving correctly.

---

## Double cursor — **does not occur**

Suspicion: if AVFoundation were capturing the hardware cursor, the output would
contain two cursors (the real one plus the composited sprite).

Checked directly by extracting frame t=15 s from both files:

- `raw_t15.png` (from `testing.mp4`) — **no cursor present**. AVFoundation's
  `capture_cursor` option is off by default and is not enabled.
- `edited_t15.png` (from `testing-edited.mp4`) — **exactly one cursor**, the
  composited sprite.

Eliminated.

---

## Off-display coordinates causing a render crash — **does not occur**

Suspicion: when the pointer moves onto the second display, logical coordinates
could go negative, and `dx as u32` on a negative `i32` would wrap to ~4e9,
blowing the bounds check and panicking into `ERR_RENDERING_FAILED (-4)`.

`renderer.rs:39-42` clamps **both** ends:

```rust
let draw_start_x = start_x.max(0);
let draw_start_y = start_y.max(0);
let draw_end_x = end_x.min(frame_width as i32);
let draw_end_y = end_y.min((frame.len() / (frame_width as usize * 4)) as i32);
```

Since `draw_start_x >= 0`, the `as u32` cast is always safe. Off-display points
cause the cursor to silently *disappear*, not crash. (Still a correctness gap —
see [01](01-coordinate-space.md) — but not a crash and not the current symptom.)

---

## The uncommitted `video.rs` diff — **cosmetic**

The working-tree modification to
`video-effects-processor/src/video.rs` consists of:

- a `rustfmt` line-wrap of the `stream_tb` binding, and
- removal of two `// ====` comment banners.

No behavioural change. Not the bug.

---

## Zero-byte and 48-byte `-edited.mp4` files — **stale**

Several historical output files are 0 bytes or 48 bytes (a bare `ftyp` box with
no frames), indicating past silent render failures. All predate January and fall
outside the agreed evidence window. Not evidence about the current pipeline.

---

## `findScreenDeviceIndex()` device counting — **works**

The index-counting loop is fragile (it increments on any line containing `]`),
but it empirically resolves to the correct device: today's recordings are
5120×2880, matching the 5K main display. Not currently broken.

Note it *does* cost 0.28 s, which matters for
[finding 2](02-timing-offset.md) — but the selection itself is correct.
