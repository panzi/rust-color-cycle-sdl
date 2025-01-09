# A Color Cycle Viewer for the terminal

Color Cycling is a technique to render images with color palette based
animations. It was used in 90ies video games. This program renders such
images to Unicode capable ANSI terminals. Windows is not supported, but
I'd accept a pull request for that.

This implementation only supports a the background layer (no overlays)
and no time based events (for now, maybe I'll add that at some later time).

This viewer reads [Living Worlds Maker](https://magrathea.onrender.com/)
files (only the background layer) or JSON files similar to what the
[Canvas Cycle](https://experiments.withgoogle.com/canvas-cycle) demo
by Joseph Huckaby uses.

[Short Demo Video](https://www.youtube.com/watch?v=QMQ93uL1Fhk)

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
| `Cursor Up` | Move view-port up by 1 pixel |
| `Cursor Down` | Move view-port down by 1 pixel |
| `Cursor Left` | Move view-port left by 1 pixel |
| `Cursor Right` | Move view-port right by 1 pixel |
| `Home` | Move view-port to left edge |
| `End` | Move view-port to right edge |
| `Ctrl`+`Home` | Move view-port to top |
| `Ctrl`+`End` | Move view-port to bottom |
| `Page Up` | Move view-port up by half a screen |
| `Page Down` | Move view-port down by half a screen |
| `Alt`+`Page Up` | Move view-port left by half a screen |
| `Alt`+`Page Down` | Move view-port right by half a screen |
