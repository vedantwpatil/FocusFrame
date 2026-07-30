# 1. Logical points composited into a physical-pixel frame

**Severity:** Critical
**Phase:** tracking → FFI → rendering
**Status:** proven by source reading *and* by measurement

---

## Symptom

The rendered cursor traces the correct *shape* of the real pointer motion, but at
half scale, confined to the top-left quarter of the video. The error grows the
further right and down the pointer actually moves.

## Cause

Mouse positions are captured in macOS **logical points** and used directly as
**physical pixel** indices into the video frame buffer. No scale factor is applied
at any stage.

### The full chain — nothing multiplies

| Step | Location | Behaviour |
|---|---|---|
| Capture (C) | `robotgo/mouse/mouse_c.h:118-124` | `location()` = `CGEventGetLocation(CGEventCreate(NULL))` → a `CGPoint` in the Quartz global coordinate space, i.e. **logical points**. `MMPointInt32FromCGPoint` truncates to int32. No scaling. |
| Capture (Go) | `robotgo/robotgo.go:628` | `Location()` calls `C.location()`, then multiplies by `ScaleF()` **only if** the package variable `Scale` is true. |
| — | `robotgo/robotgo.go:84` | `Scale bool` — declared in a `var` block, defaults to **false**. |
| — | this repo | `grep -rn "robotgo.Scale" --include="*.go" .` → **0 hits**. The project never sets it. |
| Store | `internal/tracking/mouse.go:34-35` | `mousePos.X = int16(xMouse)` — raw, unscaled. |
| FFI | `internal/video/effects.go:125-126` | `x: C.float(p.X), y: C.float(p.Y)` — straight through. |
| Rust ingest | `video.rs` `build_cursor_lookup` | copies `p.x, p.y` unchanged (only the timestamp is rebased). |
| Draw | `renderer.rs` `composite_cursor_subpixel` | `let idx = ((dy as u32 * frame_width + dx as u32) * 4) as usize;` — indexes the **physical** frame buffer. |

### The display

```
MAG274UPF:  Resolution 5120 x 2880   UI Looks like 2560 x 1440   Main Display: Yes
Color LCD:  Resolution 3024 x 1964 Retina
```

Backing scale factor = **2.0**. `ffprobe` confirms both of today's recordings are
5120×2880. So logical coordinates are consistently half of what the compositor
needs.

---

## Measurement

FFT template matching of the actual `cursor-sprite.png` against
`testing-edited.mp4` (sum-of-squared-differences over the sprite's opaque alpha
mask; low RMSE = genuine match). Twelve evenly spaced frames:

```
f    0 t= 0.00s (2082, 843) rmse= 2.00      f  660 t=11.00s (1137,  619) rmse=16.59
f   60 t= 1.00s (2082, 843) rmse= 1.91      f  780 t=13.00s ( 184,  443) rmse=10.45
f  180 t= 3.00s (1867, 888) rmse=10.52      f  900 t=15.00s ( 985,  868) rmse=19.90
f  300 t= 5.00s (1457,1366) rmse=15.20      f 1020 t=17.00s (1808, 1060) rmse=13.08
f  420 t= 7.00s (1175,1169) rmse= 3.94      f 1140 t=19.00s ( 156,   23) rmse=15.28
f  540 t= 9.00s ( 403, 541) rmse=16.05      f 1230 t=20.50s (1633, 1081) rmse= 5.01
```

Extended to a dense sweep (every 25th frame, 49 matches with RMSE < 40):

```
x[105, 2156]   y[15, 1386]        samples outside the 2560x1440 quadrant:  0 / 49
```

Frame is 5120×2880. The logical resolution is 2560×1440. **Over a 20-second
recording the pointer never once renders in the right or bottom 75% of the
frame.** That is the ×2 factor, measured.

> An earlier frame-differencing tracker reported outliers out to x=4488. That was
> noise from re-encoding artifacts. The sprite-based template match supersedes it
> and is what the numbers above come from.

---

## The fix is *not* `x * 2`

Two displays are attached. The correct transform is:

```
physical = (logical - captured_display_origin_pts) * backingScale(captured_display)
```

Three separate problems with hardcoding ×2:

1. **The origin term is absent entirely.** It happens to be `(0,0)` today only
   because the 5K panel is `Main Display: Yes`. Rearranging the displays in
   System Settings breaks a hardcoded ×2 silently.

2. **The scale is per-display.** The built-in Color LCD is 3024×1964 Retina with
   its own backing scale. If capture ever targets it, ×2 is the wrong constant.

3. **Off-display points are silently dropped, not flagged.** When the pointer
   moves onto the built-in panel its logical coordinates fall outside the
   captured display. `composite_cursor_subpixel` clamps with
   `start_x.max(0)` / `start_y.max(0)` (renderer.rs:39-40) and bounds the far
   edge with `.min(frame_width)`, so this does *not* crash — the cursor simply
   vanishes from the output for the duration. That is a correctness gap worth an
   explicit decision (clamp to edge? hide? interpolate across?) rather than an
   accident of clamping.

### Missing prerequisite

`findScreenDeviceIndex()` (`recorder.go:187`) selects the capture device by string
matching `"Capture screen 0"` in `ffmpeg -list_devices` output. It returns only an
index — it never records **which `CGDirectDisplayID`** that corresponds to. So at
the point where the transform belongs, the code does not currently know which
display's origin and scale to apply. Resolving that mapping is a prerequisite for
a correct fix, not an optional extra.

---

## Related

The cursor sprite suffers from the *same* missing factor — see
[03-cursor-sprite.md](03-cursor-sprite.md). Fix both together or the cursor ends
up correctly positioned but visually undersized.
