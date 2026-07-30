# Documentation

## Pipeline diagnosis (2026-07-30)

Root-cause investigation of the recording → processing → editing → rendering
pipeline, prompted by incorrect cursor rendering in the final output.

| # | Document | Severity |
|---|---|---|
| — | [Summary and ranked findings](diagnosis/00-summary.md) | — |
| 1 | [Coordinate space: logical points vs physical pixels](diagnosis/01-coordinate-space.md) | **Critical** |
| 2 | [Timing offset: cursor timeline leads video](diagnosis/02-timing-offset.md) | **High** |
| 3 | [Cursor sprite: size and hotspot](diagnosis/03-cursor-sprite.md) | Medium |
| 4 | [Concurrency: unsynchronized history read](diagnosis/04-concurrency.md) | Medium |
| 5 | [Latent and minor issues](diagnosis/05-latent-and-minor.md) | Low |
| 6 | [Ruled out](diagnosis/06-ruled-out.md) | — |
| 7 | [Methodology and reproduction](diagnosis/07-methodology.md) | — |

**Start here:** [diagnosis/00-summary.md](diagnosis/00-summary.md)
