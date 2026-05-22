use std::io::{Cursor, Read, Write};

use anyhow::Result;
use rayon::prelude::*;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// The required `mimetype` entry contents for an EPUB/kepub container.
const MIMETYPE: &[u8] = b"application/epub+zip";

/// One stored file inside an EPUB container (directories are not retained).
#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub data: Vec<u8>,
}

/// An EPUB's contents in memory. The `mimetype` entry is handled implicitly:
/// it is dropped on read and regenerated correctly on write.
#[derive(Debug, Default)]
pub struct Epub {
    pub entries: Vec<Entry>,
}

impl Epub {
    /// Bytes of the entry with the given path, if present.
    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.entries
            .iter()
            .find(|e| e.name == name)
            .map(|e| e.data.as_slice())
    }

    /// Replace (or insert) the entry at `name`.
    pub fn set(&mut self, name: &str, data: Vec<u8>) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.name == name) {
            entry.data = data;
        } else {
            self.entries.push(Entry {
                name: name.to_string(),
                data,
            });
        }
    }

    /// Drop entries for which `keep` returns false.
    pub fn retain<F: FnMut(&str) -> bool>(&mut self, mut keep: F) {
        self.entries.retain(|e| keep(&e.name));
    }
}

/// Parse an EPUB from raw zip bytes, decompressing entries in parallel.
pub fn read_epub(bytes: &[u8]) -> Result<Epub> {
    let count = ZipArchive::new(Cursor::new(bytes))?.len();

    let entries: Vec<Option<Entry>> = (0..count)
        .into_par_iter()
        .map(|i| -> Result<Option<Entry>> {
            let mut zip = ZipArchive::new(Cursor::new(bytes))?;
            let mut file = zip.by_index(i)?;
            let name = file.name().to_string();
            if file.is_dir() || name == "mimetype" {
                return Ok(None);
            }
            let mut data = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut data)?;
            Ok(Some(Entry { name, data }))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(Epub {
        entries: entries.into_iter().flatten().collect(),
    })
}

/// Compress a single entry into a standalone in-memory archive.
fn compress_entry(entry: &Entry) -> zip::result::ZipResult<ZipArchive<Cursor<Vec<u8>>>> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file(&entry.name, deflated)?;
    zip.write_all(&entry.data)?;
    zip.finish_into_readable()
}

/// Serialize an EPUB to zip bytes, writing `mimetype` first and uncompressed
/// as the EPUB specification requires.
///
/// Deflate compression dominates the cost, so every entry is compressed in
/// parallel into its own archive; those archives are then raw-copied (without
/// recompression) into the final container.
pub fn write_epub(epub: &Epub) -> Result<Vec<u8>> {
    let parts: Vec<ZipArchive<Cursor<Vec<u8>>>> = epub
        .entries
        .par_iter()
        .map(compress_entry)
        .collect::<zip::result::ZipResult<_>>()?;

    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buf));

        let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        zip.start_file("mimetype", stored)?;
        zip.write_all(MIMETYPE)?;

        for part in parts {
            zip.merge_archive(part)?;
        }
        zip.finish()?;
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_entries_and_normalizes_mimetype() {
        let mut epub = Epub::default();
        epub.set("OEBPS/content.opf", b"<package/>".to_vec());
        epub.set("OEBPS/ch1.xhtml", b"<html/>".to_vec());

        let bytes = write_epub(&epub).unwrap();
        let back = read_epub(&bytes).unwrap();

        assert_eq!(back.entries.len(), 2);
        assert_eq!(back.get("OEBPS/content.opf"), Some(&b"<package/>"[..]));
        assert_eq!(back.get("OEBPS/ch1.xhtml"), Some(&b"<html/>"[..]));

        // mimetype must be the first entry and uncompressed.
        let mut zip = ZipArchive::new(Cursor::new(&bytes)).unwrap();
        let first = zip.by_index(0).unwrap();
        assert_eq!(first.name(), "mimetype");
        assert_eq!(first.compression(), CompressionMethod::Stored);
    }
}
