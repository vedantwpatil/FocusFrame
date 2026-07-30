# 7. Methodology and reproduction

How each claim in this diagnosis was established, so results can be re-derived
or challenged.

---

## Evidence window

Restricted to artifacts created 2026-07-30:

```
testing.mp4              testing-edited.mp4
testing-race.mp4         testing-race-edited.mp4
```

Older debug CSVs and output videos were excluded by request and are not cited
anywhere in this diagnosis.

---

## Locating the rendered cursor: FFT template matching

The decisive measurement in [01](01-coordinate-space.md).

### Why not frame differencing

The first attempt diffed `testing-edited.mp4` against `testing.mp4` and took the
peak of a sliding-window box sum. Re-encoding introduces differences across the
whole frame, so the peak is not reliably the cursor. An 11-frame sample suggested
the cursor stayed in the top-left quadrant; extending to 61 frames produced
outliers at x=4488, y=1968 that contradicted it.

**Both results were discarded.** The frame-diff tracker is not trustworthy here.

### What was used instead

Because the sprite is composited with an "over" operator, wherever sprite alpha is
255 the output pixel equals the sprite pixel. That makes the real
`cursor-sprite.png` an exact template. Matching is sum-of-squared-differences
restricted to the sprite's opaque mask, computed via FFT:

```
SSD(x,y) = Σ_mask (I - T)²  =  corr(I², M) - 2·corr(I, M·T) + Σ M·T²
```

Each term is a cross-correlation, evaluated with `scipy.signal.fftconvolve`. Low
RMSE at the argmin confirms a genuine match rather than a coincidental minimum;
observed values were 2–20 against a 0–255 range.

```python
import sys, numpy as np, subprocess
from scipy.signal import fftconvolve
from PIL import Image
Image.MAX_IMAGE_PIXELS = None

vid, spr = sys.argv[1], sys.argv[2]
frames = [int(x) for x in sys.argv[3:]]
W, H = 5120, 2880

s = np.array(Image.open(spr).convert('RGBA')).astype(np.float64)
M = (s[:, :, 3] > 250).astype(np.float64)      # opaque mask
T = s[:, :, :3].mean(axis=2) * M               # grayscale template

def corr(I, K):
    return fftconvolve(I, K[::-1, ::-1], mode='valid')

sumT2 = (T * T * M).sum()
for f in frames:
    raw = subprocess.run(
        ['ffmpeg', '-v', 'error', '-i', vid, '-vf', f'select=eq(n\\,{f})',
         '-vframes', '1', '-f', 'rawvideo', '-pix_fmt', 'gray', '-'],
        capture_output=True).stdout
    if len(raw) < W * H:
        continue
    I = np.frombuffer(raw, dtype=np.uint8)[:W*H].reshape(H, W).astype(np.float64)
    ssd = (corr(I * I, M) - 2 * corr(I, T) + sumT2) / M.sum()
    i = np.unravel_index(np.argmin(ssd), ssd.shape)
    print(f"f{f:5d} t={f/60.0:6.2f}s  sprite_topleft=({i[1]:5d},{i[0]:5d})  "
          f"rmse={np.sqrt(ssd[i]):6.2f}")
```

Run as:

```bash
python3 tmatch.py testing-edited.mp4 internal/video/cursor-sprite.png \
    $(python3 -c "print(' '.join(str(i) for i in range(15,1240,25)))")
```

Result over 49 matches with RMSE < 40: `x[105,2156] y[15,1386]`, with **zero**
samples outside the 2560×1440 top-left quadrant of a 5120×2880 frame.

---

## Verifying the coordinate chain by source

Each hop was read rather than assumed, per the project's "don't guess an
unfamiliar API" rule:

```bash
R=$(go env GOMODCACHE)/github.com/go-vgo/robotgo@v0.110.7

# the C behind Location()
grep -rn -A10 "MMPointInt32 location()" "$R"
#  → CGEventGetLocation(CGEventCreate(NULL)) — Quartz logical points

# the Go wrapper's conditional scaling
sed -n '626,640p' "$R/robotgo.go"
#  → multiplies by ScaleF() only "if Scale || runtime.GOOS == windows"

# the default of that flag
grep -n "Scale bool" "$R/robotgo.go"        # line 84, var block, default false

# whether this project ever enables it
grep -rn "robotgo.Scale" --include="*.go" . # 0 hits
```

> Note: the `rtk` `find` wrapper did not list `internal/video/cursor-sprite.png`
> (mode 700), and `grep --include=` needs quoting under zsh. Both cost time
> during this investigation; use `ls` / quoted patterns when a file is expected
> but not found.

---

## Display configuration

```bash
system_profiler SPDisplaysDataType | grep -E "Resolution|UI Looks|Main Display|Mirror"
```

```
MAG274UPF:  Resolution: 5120 x 2880    UI Looks like: 2560 x 1440    Main Display: Yes
Color LCD:  Resolution: 3024 x 1964 Retina
```

---

## Sprite geometry

```python
from PIL import Image
import numpy as np
a = np.array(Image.open('internal/video/cursor-sprite.png').convert('RGBA'))
for t in (10, 200):
    ys, xs = np.nonzero(a[:, :, 3] > t)
    print(f"alpha>{t}: x[{xs.min()},{xs.max()}] y[{ys.min()},{ys.max()}]")
```

```
size 133x200
alpha>10 : x[7,121]  y[7,191]
alpha>200: x[14,118] y[8,179]     opaque px 8560
```

---

## Capture timing

```bash
ffprobe -v error -select_streams v:0 -show_entries frame=pkt_pts_time \
        -of csv testing.mp4
```

1243 frames over 20.733 s; median inter-frame delta 16.67 ms; 2 frames exceeding
25 ms.

Device enumeration cost:

```bash
/usr/bin/time -p ffmpeg -f avfoundation -list_devices true -i ""
```

0.28 s real. (The command exits non-zero with `Error opening input file .` — that
is the normal idiom for enumeration; the timing is the datum.)

**Not measured:** AVFoundation capture-session open latency. Obtaining it
requires actually capturing the screen. The 0.28 s figure in
[02](02-timing-offset.md) is therefore stated as a floor, not a total.

---

## Frames extracted for visual inspection

```bash
ffmpeg -ss 15 -i testing.mp4        -vframes 1 raw_t15.png
ffmpeg -ss 15 -i testing-edited.mp4 -vframes 1 edited_t15.png
```

Used to confirm the absence of a hardware cursor in the raw capture and the
presence of exactly one composited cursor in the output
([06](06-ruled-out.md)).
