# XVI: hex editor for Linux terminal

Hex editor with ncurses based user interface:
- Low resource utilization, minimum dependencies;
- Support for some VIM keyboard shortcuts (`hjkl`, `:`, `/`, etc);
- Visual diff between several files;
- Highlighting the current position and changed data;
- Insert bytes into the middle of the file;
- Cutting bytes from the middle of the file;
- Filling the range with a pattern;
- Undo/redo support;
- Search and goto;
- Customizable UI colors.

![Screenshot](https://raw.githubusercontent.com/artemsen/xvi/master/.github/screenshot1.png)

![Screenshot](https://raw.githubusercontent.com/artemsen/xvi/master/.github/screenshot2.png)

## Build

The project uses Rust and Cargo:

```
cargo build --release
```

## Install

- Arch users can install the program via [AUR](https://aur.archlinux.org/packages/xvi-git);
- AppImage is available in [Releases](https://github.com/artemsen/xvi/releases).
