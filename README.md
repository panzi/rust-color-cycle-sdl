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

Short demo video:
[![Preview screenshot of short demo video](https://i3.ytimg.com/vi/Fdk7anwM7f0/maxresdefault.jpg)](https://www.youtube.com/watch?v=Fdk7anwM7f0)

## Usage

```
Usage: color-cycle-sdl [OPTIONS] <PATHS>...

Arguments:
  <PATHS>...
          Path to a Canvas Cycle JSON file

Options:
  -f, --fps <FPS>
          Frames per second.

          Attempt to render in this number of frames per second. Actual FPS might be lower.

          [default: 60]

  -b, --blend
          Enable blend mode.

          This blends the animated color palette for smoother display.

  -o, --osd
          Enable On Screen Display.

          Displays messages when changing things like blend mode or FPS.


  -F, --full-screen
          Start in fullscreen

  -c, --cover
          Cover the window with the animation.

          Per default the animation will be contained, leading to black bars if the window
          doesn't have the same aspect ratio as the animation. With this option the
          animation is zoomed in so that it will cover the window and will crop out parts
          of the animation.

      --ilbm-column-swap
          Swap direction of 8 pixel columns.

          The current implementation of ILBM files is broken for some files and swaps the
          pixels in columns like that. I haven't figured out how do load those files
          correctly (how to detect its such a file), but this option can be used to fix the
          display of those files.

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
| `Q` | Quit program |
| `Escape` | Close full-screen or quit program |
| `O` | Toggle On Screen Display |
| `C` | Toggle zoom to cover/contain |
| `N` | Open next file |
| `P` | Open previous file |
| `1` to `9` | Open file by index |
| `0` | Open last file |
| `+` | Increase frames per second by 1 |
| `-` | Decrease frames per second by 1 |
| `F` or `F11` | Toggle full-screen |
| `W` | Toogle fast forward (10000x speed) |
| `A` | Go back in time by 5 minutes |
| `D` | Go forward in time by 5 minutes |
| `S` | Go to current time and continue normal progression |
| `Cursor Up`    | Move view-port up by 1 pixel |
| `Cursor Down`  | Move view-port down by 1 pixel |
| `Cursor Left`  | Move view-port left by 1 pixel |
| `Cursor Right` | Move view-port right by 1 pixel |
| `Ctrl`+`Cursor Up`    | Move view-port up by 5 pixel |
| `Ctrl`+`Cursor Down`  | Move view-port down by 5 pixel |
| `Ctrl`+`Cursor Left`  | Move view-port left by 5 pixel |
| `Ctrl`+`Cursor Right` | Move view-port right by 5 pixel |
