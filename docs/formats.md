# Output formats

`qrkit` renders one vector scene into any of eight output formats, with
`auto` picking a terminal one. All formats share identical geometry — the
SVG, PDF, and EPS of the same code are pixel-consistent by construction.

Pick a format with `--format <name>` or infer it from the output extension
(`-o qr.svg`, `qr.pdf`, `qr.eps`, `qr.png`, `qr.term`/`.ansi`/`.txt`,
`qr.six`/`.sixel`). Without `-o`, SVG goes to stdout.

## Vector formats

### `svg`

Hand-rolled, dependency-free SVG. Consecutive same-fill modules are grouped
into `<g>` elements; gradients use `userSpaceOnUse` coordinates; logos are
embedded as base64 data URIs. Perfect for browsers, Figma, Illustrator,
Inkscape, and laser/paper workflows. Alpha and transparent backgrounds are
fully supported.

### `pdf`

Single-page PDF 1.7, vector-only. Gradients become shading patterns,
translucency becomes ExtGState alpha (isolated per fill run), PNG logos are
embedded as image XObjects (with an alpha SMask), JPEG logos pass through
as DCTDecode streams. Ideal for print-shop handoffs.

### `eps`

PostScript Language 3 EPS. Gradients use `shfill` with prologue-defined
shading dictionaries; logos embed as ASCIIHex images. **EPS has no alpha**:
translucent colors are pre-blended against the background (white for
transparent backgrounds) — the one visual divergence from SVG/PDF.

## Raster format

### `png`

Truecolor PNG with straight alpha, rendered at `--px` pixels per module
(default 24). `--bg transparent` produces a transparent PNG. Useful as a
fallback anywhere vector isn't accepted, and as the payload for the
terminal protocols below.

## Terminal formats

| Format | Protocol | Terminals |
|---|---|---|
| `term` | ANSI truecolor half-blocks | any terminal |
| `kitty` | [kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/) | Kitty, Ghostty, WezTerm |
| `iterm` | [iTerm2 inline images](https://iterm2.com/documentation-images.html) | iTerm2, VS Code, Konsole, mintty |
| `sixel` | DEC sixel graphics | xterm (`-ti vt340`), foot, WezTerm, mintty, Windows Terminal 1.22+ |

```console
$ qrkit url "https://example.com" --format auto   # detect the terminal
$ qrkit url "https://example.com" --format kitty  # force a protocol
$ qrkit url "https://example.com" --format sixel --px 16
```

- `auto` reads the environment (`KITTY_WINDOW_ID`, `WEZTERM_EXECUTABLE`,
  `TERM`, `TERM_PROGRAM`, `LC_TERMINAL`) and falls back to `term`
  half-blocks, which work everywhere
- Image protocols embed a PNG payload (`--px` resolution); colors, eye
  colors, gradients, and logos all carry over — module *shapes* render as
  drawn (they're rasterized, not reduced to a character grid)
- `term` half-blocks pack two module rows per character cell with 24-bit
  color; gradients are sampled per module. Shapes do not apply in a
  character grid — terminal output is always square modules. Sixel has no
  alpha: translucent pixels composite over white

## Format capabilities

| Capability | svg | pdf | eps | png | kitty/iterm | sixel | term |
|---|---|---|---|---|---|---|---|
| Module shapes (dots/rounded/classy) | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✖ (square) |
| Eye shapes + separate colors | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ |
| Gradients | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ (quantized) | ✔ (per module) |
| Translucency | ✔ | ✔ | blended | ✔ | ✔ | blended | blended |
| Logo embedding | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✖ |
| Scannability gate | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ |

Every format is held to the same decode gate: the rasterizer + rqrr
round-trip tests assert that rendered output decodes back to the exact
payload across all 40 style combinations (see
[scanner safety](scanner-safety.md)).
