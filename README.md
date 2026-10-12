[![License](https://img.shields.io/github/license/Censera/palette.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-edition%202024-orange.svg)](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)

Inspect your terminal's 16-color palette and convert colors between notations. No dependencies.

## Install

```sh
cargo install --path .
```

The release profile is tuned for speed: `opt-level = 3`, fat LTO, one codegen unit, `panic = "abort"`.

## Usage

```
palette                          compact 16-color preview
palette expand [--bright]        swatches with colored names and HEX values
palette show <color> [--convert] inspect a palette entry or a color
palette let <input> <output>     convert a color (swatch shown on a terminal)
palette full [--bright]          names on each background; --bright adds bright ones
palette help
palette version

--color=<auto|always|never>      ANSI colors in output (default: auto, honors NO_COLOR)
```

`<color>` is a palette index (`09`), a name (`bright-red`), or a literal: `#db8177`, `#fa0`,
`rgb(219 129 119)`, `hsl(6deg 58% 66%)`, `hsv(...)`, `oklab(...)`, `oklch(60% 0.12 25)`.
`<output>` is `HEX`, `RGB`, `HSL`, `HSV`, `OKLab`, or `OKLCH`, in any letter case.

- A swatch is three painted spaces. `palette` shows the 16 slots; `show` shows the color three times.
- `expand` lists the normal colors with their names (drawn in their own color) and HEX values.
  `--bright` adds the bright colors, in the same format, beside them.
- `full` prints every name in its own color, first on the terminal's background and then on each of
  the eight normal backgrounds. `--bright` also tries the eight bright backgrounds.
- `show` takes any supported notation. With `--convert` it lists the color in all of them.
- `let` prints the converted value, then three swatches of the color. The swatches appear only when
  colors are on, so a pipe gets the value alone.

With colors off (piped output, `NO_COLOR`, `--color=never`) swatches become `[NN]` index labels where
there is an index, and `full` prints only the first block, since backgrounds cannot be told apart.

```
$ palette expand --bright
[00][01][02][03][04][05][06][07]
[08][09][10][11][12][13][14][15]

black    #242120   bright-black    #36312f
red      #c6685d   bright-red      #db8177
...

$ palette let '#ffffff' OKLCH | cat
oklch(100% 0 0)
```

Exit status: `0` success, `1` palette detection or conversion failure, `2` invalid usage.
Errors go to stderr.

## Where the palette comes from

| Terminal | Source |
| -------- | ------ |
| Linux virtual console (`TERM=linux`) | `/sys/module/vt/parameters/default_{red,grn,blu}` |
| kitty | `kitty.conf` and files it `include`s |
| Anything else (Unix) | OSC 4 query over `/dev/tty` (xterm, foot, alacritty, wezterm, VTE, tmux, ...) |

If the palette cannot be detected, `palette` says so and exits with status 1. It never substitutes a
default. Notes:

- The kitty source is the *configured* palette. Runtime changes or an unreloaded config can differ
  from what is active. `globinclude`, `envinclude` and `geninclude` are not followed.
- The sysfs values are the kernel's default VT palette parameters.
- `show` and `let` with a literal color need no palette source. Neither do the compact `palette`
  preview and `full`: they paint the terminal's own 16 slots.

## Color handling

Colors are kept as linear-light sRGB in `f64`, not quantized to 8 bits, so OKLab/OKLCH conversions
keep their precision. HEX, RGB, HSL and HSV cannot express colors outside the sRGB gamut: such a
color is reported as an error rather than clamped. Channels that overshoot by less than 0.05% (float
and printed-rounding noise) are clamped. Alpha is not supported.

## Terminal support

| Terminal | Palette slots | Literal colors (`show`, `let`) |
| -------- | ------------- | ------------------------------ |
| Linux console (`TERM=linux`) | yes; bright backgrounds use the blink attribute | value only, no swatch |
| true color (`COLORTERM=truecolor`/`24bit`, Windows Terminal, kitty, `*-direct`) | yes | exact |
| 256 colors (`TERM` contains `256color`) | yes | nearest of the 256 colors |
| anything else | yes | value only, no swatch |

- **Linux console.** It folds `ESC[100m`-`ESC[107m` into `ESC[40m`-`ESC[47m`, so bright swatches would look
  identical to normal ones. Bright backgrounds are drawn as `ESC[5;4Xm` instead (blink sets the bright
  bit there), which is what fastfetch does. The console also rounds 24-bit and 256-color backgrounds to
  eight dark colors, so literal colors are not drawn there.
- **`TERM` unset or `dumb`.** Colors are off in auto mode, and the palette is not queried, so nothing is
  written to the terminal.
- **Palette query (OSC 4).** It needs the foreground job of a terminal, and exits with an error in a
  background job. Settings are always restored. Linux x86_64/aarch64 use termios directly; other Unixes
  use `stty`.
- **Width.** The bright-name lines of `full` are about 100 columns wide and wrap on an 80-column terminal.

## License

[MIT](LICENSE)
