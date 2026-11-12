# Content types

Every `qrkit` subcommand produces a **standards-compliant payload** for its
content type — the string a scanner decodes and hands to the receiving app
(browser, WiFi settings, contacts, calendar). Validation happens eagerly:
invalid input fails before any QR is rendered.

All examples below assume the CLI; the library types live in
`qrkit::content` (one enum variant per type).

---

## Text

```console
$ qrkit text "hello world"
```

Free-form text, passed through verbatim. No validation beyond QR capacity.

## URL

```console
$ qrkit url "example.com/promo"        # https:// is added automatically
$ qrkit url "http://intranet.local"    # explicit schemes are kept
```

- If the input has no `://` scheme, `https://` is prefixed (CLI and WASM
  JSON; the library's `Content::Url` is strict and requires the scheme).
- Only `http` and `https` schemes are accepted; the host must be non-empty.

Payload: the URL string itself.

## Email

```console
$ qrkit email --to ada@example.com --subject "Hello" --body "First line"
```

Payload: `mailto:to?subject=…&body=…` — subject/body percent-encoded,
omitted when not given. The address must contain `@` with a dotted domain.

## SMS

```console
$ qrkit sms --number "+15551234567" --message "hi"
```

Payload: `SMSTO:number:message` (message part omitted when empty).

## Phone

```console
$ qrkit phone "+15551234567"
```

Payload: `tel:number`.

## WiFi

```console
$ qrkit wifi --ssid "Cafe Guest" --password "latte-2026"   # wpa is the default
$ qrkit wifi --ssid "OpenNet" --encryption nopass
$ qrkit wifi --ssid "Hidden" --password "pw" --encryption wpa --hidden
```

Payload: `WIFI:T:WPA;S:<ssid>;P:<password>;H:true;;` where

- `--encryption` is `wpa` (default), `wep`, or `nopass`
- WPA/WEP require a password; `nopass` forbids one
- `--hidden` adds `H:true` for hidden networks
- SSID and password escape `\ ; , : "` with a backslash (special characters
  are safe to use verbatim)

## vCard (contact)

```console
$ qrkit vcard --name "Ada Lovelace" --org "Analytical Engines" \
    --title "Mathematician" --phone "+15551234567" --email "ada@example.com" \
    --url "https://example.com/ada" --address "12 Dover St;London;;W1;UK" \
    --note "First programmer"
```

Payload: a [vCard 3.0](https://tools.ietf.org/html/rfc2426) block:

```text
BEGIN:VCARD
VERSION:3.0
FN:Ada Lovelace
N:Lovelace;Ada;;;
ORG:Analytical Engines
TITLE:Mathematician
TEL:+15551234567
EMAIL:ada@example.com
URL:https://example.com/ada
ADR:;;12 Dover St;London;;W1;UK
NOTE:First programmer
END:VCARD
```

Notes:

- `--name` is required; the structured `N` field is derived from it (last
  token becomes the family name)
- `--phone` and `--email` accept the flag multiple times for several values
- TEXT properties (`\ ; ,` and newlines) are escaped per RFC 2426; `ADR` is
  emitted as a structured value — supply it pre-joined with `;` separators
- Importing this QR adds the contact to the phone's address book

## Geo location

```console
$ qrkit geo --lat 51.5007 --lon -0.1246
```

Payload: `geo:51.5007,-0.1246` — opens the map app at that point.
Latitude is validated to −90..90, longitude to −180..180.

## Calendar event

```console
$ qrkit calendar --title "Launch party" \
    --start 20261201T180000Z --end 20261201T210000Z \
    --location "Cafe, upstairs" --description "Bring stickers"
```

Payload: a VEVENT block (`SUMMARY`/`DTSTART`/`DTEND`/`LOCATION`/
`DESCRIPTION`).

- Timestamps are strict UTC `YYYYMMDDTHHMMSSZ` (shape-validated)
- End must not be before start
- Scanning offers to add the event to the calendar
