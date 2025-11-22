# i3mac (Name liable to change)

- An attempt at recreating the Linux i3 window manager for MacOS.
- Built in Rust, and partially an experiment in creating all required FFIs manually, without using any external
  creates (bar `libc` temporarily).

## Current Features

- Currently, has the following features:
    - Windows are locked in place, and not able to be resized or moved via mouse interaction. (The inability to resize a
      window is likely to be removed.)
    - Windows can be resized via the keyboard by holding <kbd>⌘</kbd>+<kbd>⌃</kbd> and pressing any directional
      key (<kbd>← → ↑ ↓</kbd>).
    - Windows can be focused via holding <kbd>⌘</kbd>+<kbd>⌥</kbd> and pressing any directional
      key (<kbd>← → ↑ ↓</kbd>).
    - Displays are split into the concepts of physical and logical. Physical displays are detected via Core Graphics,
      whereas logical displays are created/destroyed by the use in the same way as in i3. Logical displays can be
      focused via holding <kbd>⌘</kbd>+<kbd>⌥</kbd> and pressing any number key <kbd>0-9</kbd>.
    - Windows can be moved to a different logical display via holding <kbd>⌘</kbd>+<kbd>⌥</kbd>+<kbd>⇧</kbd> and
      pressing
      any number
      key <kbd>0-9</kbd>.
    - New terminals can be opened via <kbd>⌘</kbd>+<kbd>↩︎</kbd>.
    - Containers can be split either horizontally or vertically using <kbd>⌘</kbd>+<kbd>⌥</kbd>+<kbd>V</kbd> and <kbd>
      ⌘</kbd>+<kbd>⌥</kbd>+<kbd>H</kbd> respectively.

## Command Line Arguments

- `--padding <value>`: A padding value to apply to windows.