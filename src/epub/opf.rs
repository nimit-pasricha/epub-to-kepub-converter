use std::io::Cursor;

use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, Writer};

/// Media types treated as transformable content documents.
const CONTENT_TYPES: &[&str] = &["application/xhtml+xml", "text/html"];

/// Decode `%XX` escapes in a manifest href.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                out.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Resolve a manifest `href` to a zip path, relative to the OPF's directory.
pub fn resolve_href(opf_path: &str, href: &str) -> String {
    let href = percent_decode(href.split(['#', '?']).next().unwrap_or(href));
    let mut segments: Vec<&str> = match opf_path.rsplit_once('/') {
        Some((dir, _)) => dir.split('/').filter(|s| !s.is_empty()).collect(),
        None => Vec::new(),
    };
    for part in href.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            other => segments.push(other),
        }
    }
    segments.join("/")
}

fn attr_value(e: &BytesStart, name: &[u8]) -> Option<String> {
    e.attributes().flatten().find_map(|a| {
        if a.key.local_name().as_ref() == name {
            a.unescape_value().ok().map(|v| v.into_owned())
        } else {
            None
        }
    })
}

/// Zip paths of every XHTML content document listed in the OPF manifest.
pub fn content_docs(opf_xml: &[u8], opf_path: &str) -> Vec<String> {
    let mut reader = Reader::from_reader(opf_xml);
    let mut buf = Vec::new();
    let mut docs = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if e.local_name().as_ref() == b"item" {
                    let media = attr_value(&e, b"media-type").unwrap_or_default();
                    if CONTENT_TYPES.iter().any(|t| t.eq_ignore_ascii_case(&media)) {
                        if let Some(href) = attr_value(&e, b"href") {
                            docs.push(resolve_href(opf_path, &href));
                        }
                    }
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    docs
}

/// Add `properties="cover-image"` to the manifest item that holds the cover
/// image, so Kobo shows the cover in its library. Returns the rewritten OPF
/// when a change was made, or `None` when nothing needed changing.
pub fn normalize_cover(opf_xml: &[u8]) -> Option<Vec<u8>> {
    // First pass: the cover id referenced by `<meta name="cover" content="..">`.
    let cover_id = {
        let mut reader = Reader::from_reader(opf_xml);
        let mut buf = Vec::new();
        let mut found = None;
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    if e.local_name().as_ref() == b"meta"
                        && attr_value(&e, b"name").as_deref() == Some("cover")
                    {
                        found = attr_value(&e, b"content");
                        break;
                    }
                }
                Ok(Event::Eof) | Err(_) => break,
                _ => {}
            }
            buf.clear();
        }
        found
    };

    let mut reader = Reader::from_reader(opf_xml);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();
    let mut changed = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => break,
            Ok(Event::Empty(e)) if e.local_name().as_ref() == b"item" => {
                let rewritten = rewrite_item(&e, cover_id.as_deref());
                changed |= rewritten.is_some();
                writer
                    .write_event(Event::Empty(rewritten.unwrap_or_else(|| e.borrow())))
                    .ok()?;
            }
            Ok(Event::Start(e)) if e.local_name().as_ref() == b"item" => {
                let rewritten = rewrite_item(&e, cover_id.as_deref());
                changed |= rewritten.is_some();
                writer
                    .write_event(Event::Start(rewritten.unwrap_or_else(|| e.borrow())))
                    .ok()?;
            }
            Ok(ev) => {
                writer.write_event(ev).ok()?;
            }
            Err(_) => return None,
        }
        buf.clear();
    }

    changed.then(|| writer.into_inner().into_inner())
}

/// If `item` is the cover image and lacks the `cover-image` property, return a
/// copy with `properties="cover-image"` added.
fn rewrite_item<'a>(e: &BytesStart<'a>, cover_id: Option<&str>) -> Option<BytesStart<'a>> {
    let id = attr_value(e, b"id");
    let media = attr_value(e, b"media-type").unwrap_or_default();
    let is_cover = match (cover_id, id.as_deref()) {
        (Some(cid), Some(id)) => cid == id,
        (None, Some("cover")) => true,
        _ => false,
    };
    if !is_cover || !media.starts_with("image/") {
        return None;
    }

    let existing = attr_value(e, b"properties").unwrap_or_default();
    if existing.split_whitespace().any(|p| p == "cover-image") {
        return None;
    }

    let mut out = BytesStart::new(String::from_utf8_lossy(e.name().as_ref()).into_owned());
    let mut wrote_properties = false;
    for attr in e.attributes().flatten() {
        if attr.key.local_name().as_ref() == b"properties" {
            let merged = if existing.is_empty() {
                "cover-image".to_string()
            } else {
                format!("{existing} cover-image")
            };
            out.push_attribute((
                String::from_utf8_lossy(attr.key.as_ref()).as_ref(),
                merged.as_str(),
            ));
            wrote_properties = true;
        } else {
            out.push_attribute(attr);
        }
    }
    if !wrote_properties {
        out.push_attribute(("properties", "cover-image"));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_hrefs_relative_to_opf() {
        assert_eq!(
            resolve_href("OEBPS/content.opf", "text/ch1.xhtml"),
            "OEBPS/text/ch1.xhtml"
        );
        assert_eq!(
            resolve_href("OEBPS/sub/content.opf", "../images/c.jpg"),
            "OEBPS/images/c.jpg"
        );
        assert_eq!(resolve_href("content.opf", "ch1.xhtml"), "ch1.xhtml");
        assert_eq!(
            resolve_href("OEBPS/content.opf", "a%20b.xhtml"),
            "OEBPS/a b.xhtml"
        );
    }

    #[test]
    fn lists_only_xhtml_content() {
        let opf = br#"<package><manifest>
            <item id="c1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
            <item id="css" href="style.css" media-type="text/css"/>
            <item id="c2" href="ch2.html" media-type="text/html"/>
          </manifest></package>"#;
        let docs = content_docs(opf, "OEBPS/content.opf");
        assert_eq!(docs, vec!["OEBPS/ch1.xhtml", "OEBPS/ch2.html"]);
    }

    #[test]
    fn adds_cover_image_property() {
        let opf = br#"<package><metadata>
            <meta name="cover" content="cover-img"/>
          </metadata><manifest>
            <item id="cover-img" href="cover.jpg" media-type="image/jpeg"/>
          </manifest></package>"#;
        let out = normalize_cover(opf).expect("should change");
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains(r#"properties="cover-image""#), "{text}");
    }

    #[test]
    fn leaves_opf_without_cover_alone() {
        let opf = br#"<package><manifest>
            <item id="c1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
          </manifest></package>"#;
        assert!(normalize_cover(opf).is_none());
    }
}
