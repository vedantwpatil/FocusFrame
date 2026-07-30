# 3. Cursor sprite: size and hotspot

**Severity:** Medium
**Phase:** rendering
**Status:** confirmed by measurement of the asset and visual inspection

> **This is not an independent bug.** It is the same missing `backingScale`
> factor as [01-coordinate-space.md](01-coordinate-space.md). Fixing coordinates
> without also handling the sprite yields a correctly-positioned cursor that is
> now visually *undersized* relative to a 2× UI.

---

## 3a. Sprite is drawn 1:1 into a 2× frame

`internal/video/cursor-sprite.png` is **133 × 200 px**. `load_cursor_sprite`
loads it unscaled and `composite_cursor_subpixel` draws it at native size into a
5120×2880 physical frame.

133 × 200 physical pixels ÷ 2.0 backing scale = **66 × 100 logical points**.

A real macOS arrow pointer is roughly 32 × 32 logical points. The rendered cursor
is several times too large in area, which is clearly visible in the extracted
frame `edited_t15.png`.

The sprite needs to be scaled by the same display-derived factor that scales the
coordinates — not by an independent constant, or the two will drift apart when the
capture target changes.

---

## 3b. Anchor point is the sprite corner, not the pointer tip

`composite_cursor_subpixel` (renderer.rs:33-36) anchors the sprite's **top-left
corner** at the supplied position:

```rust
let start_x = x.floor() as i32;
let start_y = y.floor() as i32;
let end_x = start_x + cursor.width as i32 + 1;   // +1 for bilinear spill
let end_y = start_y + cursor.height as i32 + 1;
```

But the sprite has transparent padding:

| Threshold | Bounding box |
|---|---|
| `alpha > 10` | x `[7, 121]`, y `[7, 191]` |
| `alpha > 200` | x `[14, 118]`, y `[8, 179]` |

Opaque pixel count: 8560 of 26600.

So the visible pointer tip renders roughly **7–14 px down and right** of the
reported position. Small next to the ×2 error, but it is an accidental offset
arising from asset padding rather than a declared hotspot.

## Fix direction

Introduce an explicit hotspot (e.g. `hotspot_x`, `hotspot_y` on `CursorSprite`)
and subtract it at draw time, so the anchor is the pointer tip regardless of how
the PNG is cropped or rescaled. Otherwise re-exporting the asset with different
padding silently moves the cursor.
