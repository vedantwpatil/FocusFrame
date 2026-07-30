# Pipeline diagnosis — summary

**Date:** 2026-07-30
**Scope:** recording → processing → editing → rendering
**Evidence window:** artifacts from 2026-07-30 only (`testing.mp4`,
`testing-edited.mp4`, `testing-race.mp4`, `testing-race-edited.mp4`). Older
files were excluded by request.

---

## The short version

Two independent defects compound, which is why the output looks inexplicable
rather than like a clean offset:

1. **The cursor is drawn in the wrong place.** Mouse coordinates are captured in
   macOS *logical points* and composited into a *physical pixel* frame with no
   scale factor applied anywhere in the chain. On the 5K main display
   (5120×2880 physical, 2560×1440 logical) that is a uniform ×2 error — the
   rendered cursor never leaves the top-left quarter of the frame.

2. **The cursor is drawn too early.** The cursor clock starts before `ffmpeg`
   is even launched, but the renderer aligns cursor t=0 to video frame 0. All
   the pre-roll pointer motion gets glued onto the front of the video, so the
   cursor arrives at targets before the UI reacts.

Fixing only one of these still leaves visibly wrong output.

---

## Ranked findings

| # | Finding | Severity | Phase | Survives fix of #1? |
|---|---|---|---|---|
| 1 | [Logical points composited into physical-pixel frame; no display origin either](01-coordinate-space.md) | **Critical** | tracking → rendering | — |
| 2 | [Cursor timeline leads video by ≥0.28 s](02-timing-offset.md) | **High** | recording → rendering | Yes |
| 3 | [Sprite drawn 1:1 in physical px; unintended hotspot offset](03-cursor-sprite.md) | Medium | rendering | Partially — see note |
| 4 | [`GetCursorHistory()` reads slice with no lock](04-concurrency.md) | Medium | recording | Yes |
| 5 | [Stride ignored (latent), sub-1 s timestamp ×1000, progress %, CWD-relative sprite path, leaked hook goroutine](05-latent-and-minor.md) | Low | various | Yes |

> **Note on #3:** the sprite size is *not* an independent issue — it is the same
> missing `backingScale` factor. A fix that scales coordinates but leaves the
> sprite alone produces a correctly-positioned cursor that is now visually
> undersized against a 2× UI. Both must be addressed together.

---

## Confidence

Finding #1 is proven twice, independently:

- **By source reading** — there is no multiplication by any scale factor at any
  point between `robotgo.Location()` and `composite_cursor_subpixel()`. Traced
  through `mouse_c.h` → `robotgo.go` → `mouse.go` → `effects.go` → `video.rs` →
  `renderer.rs`. See [01](01-coordinate-space.md).
- **By measurement** — FFT template matching of the real sprite PNG against
  `testing-edited.mp4`: **49 of 49** sampled frames place the cursor inside the
  top-left 2560×1440 region of a 5120×2880 frame. See
  [07](07-methodology.md).

Finding #2 has a **measured floor** of 0.28 s (device enumeration) but the true
value is larger; the AVFoundation capture-session open time was not cleanly
measured.

Items in [06-ruled-out.md](06-ruled-out.md) were actively tested and eliminated —
notably capture quality, VFR→CFR conversion, and a suspected double-cursor.

---

## What was not done

No code changes were made. The request was diagnosis. Suggested fix directions
are noted in each document but nothing was implemented.
