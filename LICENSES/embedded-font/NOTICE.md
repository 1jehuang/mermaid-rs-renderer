# Embedded font — DejaVu Sans (subset)

`assets/DejaVuSans-subset.ttf` is a subset of **DejaVu Sans** embedded at
compile time (`include_bytes!`) to give the renderer deterministic text metrics
with zero filesystem I/O on every platform (no `load_system_fonts`, no font
cache). See `src/embedded_font.rs`.

## Provenance

- **Source:** DejaVu Sans (`DejaVuSans.ttf`), the DejaVu fonts project.
- **Subset coverage:** Basic Latin + Latin-1 Supplement + common punctuation /
  symbols (created with `fonttools pyftsubset`, hinting dropped — advance widths
  in `hmtx` are unaffected, so layout metrics stay exact).
- **Family name preserved:** `DejaVu Sans` (`unitsPerEm = 2048`).

## License

DejaVu Sans is distributed under the **Bitstream Vera Fonts** license plus the
**Arev Fonts** copyright (DejaVu changes are public domain) — a permissive, free
license. It is **not** the SIL Open Font License. The full text is in
`DejaVu-LICENSE.txt`; its copyright and permission notices are retained there as
the license requires.
