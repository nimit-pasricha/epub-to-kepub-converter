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
| `-i, --in-place` | Replace each source `.epub` with its `.kepub.epub` (the original is deleted). |
| `-o, --output <DIR>` | Write every converted file into one flat directory. |
| `-f, --overwrite` | Re-convert even when the target `.kepub.epub` already exists. |

By default `book.epub` becomes `book.kepub.epub` alongside the original. Files
whose target already exists are skipped (reported on the console) unless
`--overwrite` is given. A progress bar is shown while converting.

### Examples

```
kepub book.epub                  # -> book.kepub.epub next to it
kepub ~/Books                    # convert every .epub under ~/Books
kepub ~/Books -i                 # replace the originals (--in-place)
kepub ~/Books -o ~/Kobo          # collect all kepubs in one folder (--output)
kepub ~/Books -f                 # re-convert, overwriting existing files (--overwrite)
```

## Building

The tool is written in pure Rust and builds on Linux, macOS, and Windows.

```
cargo build --release
```

On Linux the binary is at `target/release/kepub`, on Windows at
`target\release\kepub.exe`.

## Performance notes

Conversion is parallelized across books, across the content documents within
each book, and across zip (de)compression. On a typical multi-core machine a
book converts in roughly a tenth of a second.

### Speed vs. file size

The dominant remaining cost is deflate compression when re-packing the zip.
Entries are compressed at the `zip` crate's default deflate level, which
balances speed against output size.

If you ever need conversion to be even faster and don't mind larger output,
this is the knob: in `compress_entry` (`src/epub/zip_io.rs`), the
`SimpleFileOptions` can be given an explicit `.compression_level(Some(n))`.
A low level such as `Some(1)` compresses roughly 2–3x faster but produces
`.kepub.epub` files about 5–10% larger. The default is left in place because
file size usually matters more than shaving milliseconds.

## Testing

```
cargo test
```

## License

MIT
