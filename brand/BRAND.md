# OpenSkopos brand

## The mark

The mark is a Greek capital sigma (Σ, the first letter of *skopos*) with an
orange dot in its open side. The sigma gathers everything a team agreed, and
the dot is the point it all serves: the purpose.

- Strokes are square-ended with sharp (mitred) corners, to match the flat
  stroke ends of Inter and JetBrains Mono.
- There are two cuts. Use `mark-*` above 32 px. Use `mark-small-*` (heavier
  stroke, larger dot) at 32 px and below, and for the favicon.

## The name

Write the name as **OpenSkopos**: one word, with capital O and capital S. Use
the same form in the logo, in text and in code comments. Do not write
"Openskopos", "OPENSKOPOS" (except in the terminal banner below), or "Open
Skopos".

Other names:

- The CLI is `skopos`.
- Data files are `<kind>.skopos.json` and `<kind>.skopos.yaml`.
- The site is openskopos.org.

## The logo

The logo is the mark followed by the wordmark "OpenSkopos" in Inter. "Open"
is Inter Regular (400) and "Skopos" is Inter Medium (500), both in the ink
colour.

- The mark is 1.3 times the cap height of the wordmark, and its centre is on
  the cap-height midline. The dot then lines up with the centre of the capital
  O.
- The gap between the dot and the O is about one third of the cap height.
- Keep clear space around the logo equal to the cap height on every side.
- Do not change the colours, recolour only the dot, stretch the logo, add
  effects, or set the wordmark in another font.

`logo-*.svg` contains live text. Inter must be installed to see it correctly.
Convert the text to outlines before you use the file outside the web.

## Colours

| Use | Light | Dark |
|---|---|---|
| Ink (sigma and wordmark) | `#242433` | `#FAFAFA` |
| Dot | `#E8512F` | `#FF6848` |
| Background | Cream `#FFF7EA` or white | Zinc `#18181B` |

The light dot is a deeper orange than `#FF6848`, so that it keeps a contrast
of 3:1 or more against cream. Use `mark-mono.svg` where only one colour is
possible. It uses `currentColor` for the sigma and the dot.

## Fonts

- Inter for the wordmark, the site and the specification.
- JetBrains Mono for the terminal, code and CLI screenshots.

## In the terminal

The typed form of the mark is `Σ•`: the Greek capital sigma (U+03A3) and a
bullet (U+2022), with the bullet in orange where colour is available. If the
terminal cannot show Unicode, print `skopos` and no mark.

```
Σ• OpenSkopos 0.1.0
   skopos check: 3 files ok
```

## Files

| File | Use |
|---|---|
| `mark-light.svg`, `mark-dark.svg` | Mark above 32 px |
| `mark-small-light.svg`, `mark-small-dark.svg` | Mark at 32 px and below |
| `mark-mono.svg` | One colour, follows `currentColor` |
| `favicon.svg` | Browser tab. It changes to the dark colours when the system is in dark mode. |
| `icon-light.svg`, `icon-dark.svg` | 512 px square: GitHub organization avatar, npm, social |
| `logo-light.svg`, `logo-dark.svg` | Horizontal logo for the site header and the README |
| `avatar-square-black.svg`, `avatar-square-zinc.svg` (and `-512.png`) | Square corners, for places that round the corners themselves, such as the GitHub organization avatar |
