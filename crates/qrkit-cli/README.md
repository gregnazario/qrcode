# qrkit-cli

`qrkit` — the command-line face of the [qrkit](https://github.com/gregnazario/qrcode)
QR generator. Nine content subcommands (text, url, email, sms, phone, wifi,
vcard, geo, calendar), full styling flags, and output to SVG / PDF / EPS /
PNG plus four terminal formats (`term` ANSI half-blocks, the kitty and
iTerm2 image protocols, sixel, and `auto` detection).

```console
$ qrkit url "https://example.com" --shape dots --eye circle \
    --fg "#0d47a1" --gradient linear --gradient-to "#42a5f5" -o poster.pdf
$ qrkit wifi --ssid "Cafe Guest" --password "latte-2026" -o menu.eps
$ qrkit url "https://example.com" --format auto
```

Install: `cargo install qrkit-cli` · Full CLI reference:
<https://github.com/gregnazario/qrcode/blob/main/docs/cli-reference.md>
