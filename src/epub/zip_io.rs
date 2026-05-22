use std::io::{Cursor, Read, Write};

use anyhow::Result;
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

/// Parse an EPUB from raw zip bytes.
pub fn read_epub(bytes: &[u8]) -> Result<Epub> {
    let mut zip = ZipArchive::new(Cursor::new(bytes))?;
    let mut entries = Vec::new();
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        if file.is_dir() {
            continue;
        }
        let name = file.name().to_string();
        if name == "mimetype" {
            continue;
        }
        let mut data = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut data)?;
        entries.push(Entry { name, data });
    }
    Ok(Epub { entries })
}

/// Serialize an EPUB to zip bytes, writing `mimetype` first and uncompressed
/// as the EPUB specification requires.
pub fn write_epub(epub: &Epub) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buf));

        let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        zip.start_file("mimetype", stored)?;
        zip.write_all(MIMETYPE)?;

        let deflated =
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for entry in &epub.entries {
            zip.start_file(&entry.name, deflated)?;
            zip.write_all(&entry.data)?;
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
