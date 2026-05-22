use html5ever::tendril::TendrilSink;
use html5ever::{parse_document, ParseOpts};
use markup5ever_rcdom::RcDom;

/// A parsed XHTML content document.
pub struct Document {
    pub dom: RcDom,
    /// The leading `<?xml ... ?>` declaration, kept verbatim for re-emission
    /// (the HTML parser would otherwise turn it into a comment).
    pub xml_declaration: Option<String>,
}

/// Parse XHTML bytes with the error-tolerant HTML5 parser.
pub fn parse(bytes: &[u8]) -> Document {
    let text = decode_utf8(bytes);
    let (xml_declaration, rest) = split_xml_declaration(&text);
    let dom = parse_document(RcDom::default(), ParseOpts::default())
        .from_utf8()
        .read_from(&mut rest.as_bytes())
        .expect("RcDom parsing is infallible");
    Document {
        dom,
        xml_declaration,
    }
}

/// Decode bytes as UTF-8, dropping a leading byte-order mark if present.
fn decode_utf8(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    String::from_utf8_lossy(bytes).into_owned()
}

/// Split off a leading `<?xml ... ?>` declaration, returning it and the rest.
fn split_xml_declaration(text: &str) -> (Option<String>, &str) {
    let trimmed = text.trim_start();
    let offset = text.len() - trimmed.len();
    if trimmed.starts_with("<?xml") {
        if let Some(end) = trimmed.find("?>") {
            return (
                Some(trimmed[..end + 2].to_string()),
                &text[offset + end + 2..],
            );
        }
    }
    (None, text)
}
