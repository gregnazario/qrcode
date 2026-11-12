# Scanner safety

A QR code that looks good but doesn't scan is decoration. `qrkit` treats
decodability as a contract, enforced by tests — this page explains the
design decisions behind it and the one switch that opts out.

## The decode gate

The test suite rasterizes every styled scene with the production
rasterizer, decodes it with [rqrr](https://crates.io/crates/rqrr), and
asserts the **exact payload** round-trips. This runs for all 40 module-shape
× eye-shape × logo combinations at multiple render sizes, plus the terminal
half-block format and every terminal image protocol. If a style combination
fails to decode, it's a bug — geometry changes until it passes.

## Eye geometry is decode-capped

Decoders find a QR by matching its three **finder patterns** ("eyes") — a
7×7 pattern with an exact 1:1:3:1:1 dark/light ratio. That requires the
finder's corner modules to be dark, which pure geometry fights:

- A circle inscribed in the 7×7 grid can never reach the corner module
  centers (distance √2·3 ≈ 4.24 > radius 3.5)
- Rounded corners above radius ≈ 1.707 (= 0.5/(1 − 1/√2)) leave the corner
  module centers unpainted

So the shipped geometry is:

- **rounded / leaf** eyes: corner radii capped at 1.5 (of 7), with small
  corner nubs restoring the full 7×7 corner extent
- **circle** eyes: the circle plus small corner tabs completing the finder
  corners

These caps came from the decode matrix itself — the original larger radii
failed 24 of 40 style combinations and were tuned until everything decoded.

## `--decorative-eyes`: the documented opt-out

If you prefer the pure shapes — smooth circles, bigger rounding, no tabs or
nubs — `--decorative-eyes` (or the *decorative* checkbox on the web app)
restores the original geometry. Phone cameras are often forgiving; strict
decoders (and our own test gate) are not. The style warns:

> decorative eyes: finder corners are not scanner-guaranteed; strict readers
> may fail to decode

Test your target scanner on a real example before printing decorative output.

## Dot radius

`--shape dots` accepts radii 0.1–0.5, but radii below **0.4** shrink the
modules enough that stricter decoders can fail to find the timing pattern —
the style warns below 0.4 for the same reason.

## Logos force ECC High

A logo covers modules in the center of the code. Error correction level
**High** (30% recoverable) is what makes that safe, so any logo forces ECC
H (a warning notes this if you asked for less). The covered modules are
cleared rather than overdrawn, keeping the geometry clean. Keep
`--logo-size` within the validated 0.1–0.33 range.

## Contrast tips

- The default white background exists so codes scan off dark surfaces and
  dark terminal themes — keep it unless the medium really demands otherwise
- Light gradients on light backgrounds can drop module contrast below what
  decoders tolerate; prefer gradient endpoints that stay dark
- The decode gate covers the built-in styles; exotic custom fills are your
  responsibility (the library exposes `StyleConfig` if you want to build
  and decode-test your own)
