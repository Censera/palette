[![License](https://img.shields.io/github/license/Censera/palette.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-edition%202024-orange.svg)](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)

Print ANSI terminal colors to stdout. Useful for verifying what your terminal actually renders, checking 24-bit color support, and picking escape codes for scripts.

## Install

```r
cargo install --path .
```

## Usage

```ts
palette [OPTION]

Options:
  (no flag)          16-color swatch (standard + bright blocks)
  -d, --detailed     Swatch with color names
  -l, --long         Foreground samples across alternating backgrounds
  -L, --long-bright  Same as --long with bright background colors
      --rgb R,G,B    Swatch for a specific 24-bit RGB color
  -h, --help
```

## Examples

```ts
palette
palette --detailed
palette --long
palette --long-bright
palette --rgb 255,0,128
```

**Standard 16-color swatch:**

![](res/palette.png)

**Detailed view:**

![](res/palette-d.png)

**Long view:**

![](res/palette-l.png)

**Long view with bright backgrounds:**

![](res/palette-L.png)

**RGB swatch:**

![](res/palette--rgb.png)

ANSI support is enabled on startup for Windows terminals that don't default to it (`SetConsoleMode`). On terminals where ANSI is unavailable, output falls back to plain text.

## License

[MIT](LICENSE)
