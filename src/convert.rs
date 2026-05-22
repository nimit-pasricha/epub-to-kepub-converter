use anyhow::{Context, Result};
use rayon::prelude::*;

use crate::epub::{container, opf, read_epub, write_epub, Epub};
use crate::html;

/// Non-content files that are dropped from the output container.
const JUNK: &[&str] = &[
    "calibre_bookmarks.txt",
    "iTunesMetadata.plist",
    "iTunesArtwork",
    ".DS_Store",
    "Thumbs.db",
];

fn is_junk(name: &str) -> bool {
    name.starts_with("__MACOSX/")
        || name.starts_with("$RECYCLE.BIN/")
        || JUNK
            .iter()
            .any(|j| name == *j || name.ends_with(&format!("/{j}")))
}

fn is_content_ext(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".xhtml") || lower.ends_with(".html") || lower.ends_with(".htm")
}

/// Every XHTML-looking entry in the container, used when the OPF manifest is
/// missing or unusable.
fn fallback_docs(epub: &Epub) -> Vec<String> {
    epub.entries
        .iter()
        .filter(|e| is_content_ext(&e.name))
        .map(|e| e.name.clone())
        .collect()
}

/// Convert raw EPUB bytes into a Kobo kepub: inject koboSpans and wrapper divs
/// into every content document, normalize the cover, and re-pack the zip.
pub fn convert_epub(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut epub = read_epub(bytes).context("reading EPUB container")?;
    epub.retain(|name| !is_junk(name));

    let opf_path = epub
        .get("META-INF/container.xml")
        .and_then(container::find_opf_path)
        .or_else(|| {
            container::FALLBACK_OPF_PATHS
                .iter()
                .find(|p| epub.get(p).is_some())
                .map(|p| p.to_string())
        });

    let content_docs: Vec<String> = match &opf_path {
        Some(path) => {
            let docs = epub
                .get(path)
                .map(|xml| opf::content_docs(xml, path))
                .unwrap_or_default();
            if docs.is_empty() {
                fallback_docs(&epub)
            } else {
                docs
            }
        }
        None => fallback_docs(&epub),
    };

    // Transform every content document in parallel. Each document is parsed,
    // rewritten, and serialized within a single closure, so the (non-Send)
    // DOM never crosses a thread boundary.
    let inputs: Vec<(String, Vec<u8>)> = content_docs
        .iter()
        .filter_map(|path| epub.get(path).map(|raw| (path.clone(), raw.to_vec())))
        .collect();

    let outputs: Vec<(String, Vec<u8>)> = inputs
        .par_iter()
        .map(|(path, raw)| {
            let doc = html::parse(raw);
            html::wrap::transform(&doc);
            html::kobospan::transform(&doc);
            (path.clone(), html::serialize(&doc).into_bytes())
        })
        .collect();

    for (path, data) in outputs {
        epub.set(&path, data);
    }

    if let Some(path) = &opf_path {
        if let Some(updated) = epub.get(path).and_then(opf::normalize_cover) {
            epub.set(path, updated);
        }
    }

    write_epub(&epub).context("writing kepub container")
}
