# kepub

A command-line tool that recursively converts EPUB files into Kobo's enhanced
**kepub** format.

A kepub is a regular EPUB whose content documents have been augmented with
sentence-level `koboSpan` tags and Kobo's `book-columns` / `book-inner`
pagination wrapper. On a Kobo e-reader this enables much faster page rendering,
accurate page numbers, and reading statistics. `kepub` performs the full
conversion — not just a file rename — and writes files with the `.kepub.epub`
extension that Kobo expects.

Conversion is heavily parallelized — books, the content documents within each
book, and zip (de)compression all run across every available CPU core.

## Usage

```
kepub <INPUT> [OPTIONS]
```

`<INPUT>` is an `.epub` file, or a directory that is searched recursively for
`.epub` files.

| Option | Effect |
| ------ | ------ |
| `--in-place` | Replace each source `.epub` with its `.kepub.epub` (the original is deleted). |
| `-o, --output <DIR>` | Write every converted file into one flat directory. |
| `--overwrite` | Re-convert even when the target `.kepub.epub` already exists. |

By default `book.epub` becomes `book.kepub.epub` alongside the original. Files
whose target already exists are skipped (reported on the console) unless
`--overwrite` is given. A progress bar is shown while converting.

### Examples

```
kepub book.epub                  # -> book.kepub.epub next to it
kepub ~/Books                    # convert every .epub under ~/Books
kepub ~/Books --in-place         # replace the originals
kepub ~/Books -o ~/Kobo          # collect all kepubs in one folder
```

## Building

Linux (native):

```
cargo build --release
# -> target/release/kepub
```

Windows, cross-compiled from Linux with [`cross`](https://github.com/cross-rs/cross)
(requires Docker or Podman):

```
cargo install cross
cross build --release --target x86_64-pc-windows-gnu
# -> target/x86_64-pc-windows-gnu/release/kepub.exe
```

Alternatively, build for Windows with the MinGW toolchain instead of Docker:

```
rustup target add x86_64-pc-windows-gnu
# install MinGW, e.g. on Debian/Ubuntu: apt install gcc-mingw-w64-x86-64
cargo build --release --target x86_64-pc-windows-gnu
```

## Testing

```
cargo test
```

## License

MIT
