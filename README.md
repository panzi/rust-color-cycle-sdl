# A Color Cycle Viewer

Color Cycling is a technique to render images with color palette based
animations. It was used in 90ies video games.

This implementation only supports a the background layer (no overlays)
including time of day shifts, but no time based events (for now, maybe
I'll add that at some later time).

This viewer reads [Living Worlds Maker](https://magrathea.onrender.com/)
files (only the background layer) or JSON files similar to what the
[Canvas Cycle](https://experiments.withgoogle.com/canvas-cycle) demo
by Joseph Huckaby uses.

## Usage

```
Usage: color-cycle [OPTIONS] <PATH>...

Arguments:
  <PATH>...
          Path to a Canvas Cycle JSON file

Options:
  -f, --fps <FPS>
          Frames per second.

          Attempt to render in this number of frames per second. Actual FPS might be lower.

          [default: 25]

  -b, --blend
          Enable blend mode.

          This blends the animated color palette for smoother display.

  -o, --osd
          Enable On Screen Display.

          Displas messages when changing things like blend mode or FPS.


      --help-hotkeys
          Show list of hotkeys

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

## Hotkeys

| Hotkey | Description |
| :----- | :---------- |
| `B` | Toggle blend mode |
| `Q` or `Escape` | Quit program |
| `O` | Toggle On Screen Display of message |
| `N` | Open next file |
| `P` | Open previous file |
| `1` to `9` | Open file by index |
| `0` | Open last file |
| `+` | Increase frames per second by 1 |
| `-` | Decrease frames per second by 1 |
| `F` | Toggle full-screen |
| `W` | Toogle fast forward (10000x speed). |
| `A` | Go back in time by 5 minutes. |
| `D` | Go forward in time by 5 minutes. |
| `S` | Go to current time and continue normal progression. |
