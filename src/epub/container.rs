use quick_xml::events::Event;
use quick_xml::Reader;

/// Conventional OPF locations to try when `container.xml` is missing.
pub const FALLBACK_OPF_PATHS: &[&str] = &["content.opf", "OEBPS/content.opf"];

/// Locate the OPF package document from `META-INF/container.xml` by reading
/// the first `<rootfile full-path="...">`.
pub fn find_opf_path(container_xml: &[u8]) -> Option<String> {
    let mut reader = Reader::from_reader(container_xml);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if e.local_name().as_ref() == b"rootfile" {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"full-path" {
                            return attr
                                .unescape_value()
                                .ok()
                                .map(|v| v.into_owned());
                        }
                    }
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_rootfile_path() {
        let xml = br#"<?xml version="1.0"?>
            <container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
              <rootfiles>
                <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
              </rootfiles>
            </container>"#;
        assert_eq!(find_opf_path(xml).as_deref(), Some("OEBPS/content.opf"));
    }
}
