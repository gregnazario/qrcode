# CLI reference

```
qrkit [OPTIONS] <COMMAND>
```

One subcommand per [content type](content-types.md); styling and output
options are shared. Output goes to `-o`/`--output` (format inferred from
the extension) or stdout (SVG) / the terminal (terminal formats).

## Global options

| Flag | Values / default | Description |
|---|---|---|
| `-o`, `--output <PATH>` | — | Output file; format from extension: `svg` `pdf` `eps` `png` `term`/`ansi`/`txt` `six`/`sixel`. Omit for stdout |
| `--format <FMT>` | `svg` `pdf` `eps` `term` `png` `kitty` `iterm` `sixel` `auto` | Force the format (overrides the extension); see [formats](formats.md) |
| `--px <N>` | 1..=256, default 24 | Pixels per module for raster formats (`png` `kitty` `iterm` `sixel`) |
| `--ecc <LEVEL>` | `low` `medium` `quartile` `high`, default `quartile` | Error correction (a logo forces `high`) |
| `--version <V>` | 1..=40, default auto | Fixed QR symbol version |
| `--margin <M>` | modules, default 4 | Quiet-zone margin |
| `--shape <S>` | `square` `rounded` `dots` `classy`, default `square` | Module shape |
| `--dot-radius <R>` | 0.1..=0.5, default 0.5 | Dot radius for `--shape dots` (below 0.4 warns — see [scanner safety](scanner-safety.md)) |
| `--eye <S>` | `square` `rounded` `circle` `leaf`, default `square` | Eye shape |
| `--decorative-eyes` | flag | Pure-shape eyes without scanner-safety tabs/nubs (warns) |
| `--fg <HEX>` | default `#000000` | Module fill; 3/4/6/8-digit hex |
| `--bg <HEX\|transparent>` | default `#ffffff` | Background; `transparent` supported everywhere except sixel (composites to white) |
| `--gradient <KIND>` | `linear` `radial` | Gradient fill from `--fg` to `--gradient-to` (implied `linear` when only `--gradient-to` is given) |
| `--gradient-to <HEX>` | — | Gradient end color |
| `--gradient-angle <DEG>` | default 0 | Linear gradient angle (0 = left→right, 90 = top→bottom) |
| `--eye-color <HEX>` | default: follows `--fg`/gradient | Eye ring color |
| `--eye-inner-color <HEX>` | default: module fill | Eye inner dot color |
| `--logo <PATH>` | PNG/JPEG file | Center logo (clears covered modules) |
| `--logo-size <R>` | 0.1..=0.33, default 0.25 | Logo size as fraction of the code |
| `--logo-padding <P>` | modules, default 1.0 | Empty ring around the logo |
| `--preview` | flag | Unicode half-block preview on stderr (same matrix as the output, so `--version`/`--ecc` are honored) |
| `-h`, `--help` / `-V` | | Help / version |

## Subcommands

Every subcommand accepts all global options.

| Command | Arguments |
|---|---|
| `text <TEXT>` | free text |
| `url <URL>` | `https://` added when the scheme is missing |
| `email` | `--to` `--subject` `--body` |
| `sms` | `--number` `--message` |
| `phone <NUMBER>` | |
| `wifi` | `--ssid` `--password` `--encryption wpa\|wep\|nopass` `--hidden` |
| `vcard` | `--name` (required) `--org` `--title` `--phone` (repeatable) `--email` (repeatable) `--url` `--address` `--note` |
| `geo` | `--lat` `--lon` |
| `calendar` | `--title` `--start` `--end` (`YYYYMMDDTHHMMSSZ`) `--location` `--description` |

Validation is eager — bad input (malformed email, out-of-range coordinates,
inverted dates, unknown enums) exits 1 with `error: …` before anything is
rendered. Style warnings (logo ECC bump, decorative eyes, small dots)
print to stderr as `warning: …` on every run.

## Examples

```console
# Basic URL, print to stdout as SVG
qrkit url "https://example.com"

# Styled poster: gradient, dots, circle eyes, logo, PDF for print
qrkit url "https://example.com/promo" \
    --shape dots --eye circle \
    --fg "#0d47a1" --gradient-to "#42a5f5" --gradient-angle 45 \
    --logo assets/logo.png --logo-size 0.25 \
    -o poster.pdf

# WiFi card for a cafe menu (EPS for the print shop)
qrkit wifi --ssid "Cafe Guest" --password "latte-2026" -o menu.eps

# Contact card on a conference badge
qrkit vcard --name "Ada Lovelace" --org "Analytical Engines" \
    --email ada@example.com --phone "+15551234567" -o badge.svg

# Terminal: best image protocol for the current terminal
qrkit url "https://example.com" --format auto

# Terminal: inline image for kitty/Ghostty at double resolution
qrkit url "https://example.com" --format kitty --px 48

# Preview before writing (any format)
qrkit text "hi" --preview -o qr.svg
```

## Exit codes

- `0` — success (warnings may have been printed to stderr)
- `1` — validation/render/write error (`error: …` on stderr)
- `2` — argument parsing error (clap)
