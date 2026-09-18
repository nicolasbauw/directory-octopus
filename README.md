# Directory Octopus

A modern take on Amiga's Directory Opus 4: a retro-styled, dual-pane file
manager built in Rust with [egui](https://github.com/emilk/egui), featuring
a bitmap Topaz font and Workbench-style 3D bevels.

*(F1/F2/F3 switch between zoom levels.)*

## Building

```sh
cargo build --release
```

## Installing on Arch Linux

Two PKGBUILDs are provided in `packaging/`:

- `PKGBUILD` builds the latest tagged release.
- `PKGBUILD-git` builds directly from the latest commit on `master`.

```sh
cd packaging
makepkg -si -p PKGBUILD      # tagged release
# or
makepkg -si -p PKGBUILD-git  # git version
```

Either one installs the `directory-octopus` binary, its `.desktop` entry,
and its icon.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE)
at your option.
