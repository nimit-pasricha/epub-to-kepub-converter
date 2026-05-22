use markup5ever::Attribute;
use markup5ever_rcdom::{Handle, NodeData};

use super::parse::Document;

/// HTML void elements: self-closed in polyglot XHTML output.
const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// Elements whose text content must not be entity-escaped.
const RAW_TEXT: &[&str] = &["script", "style"];

const XHTML_NS: &str = "http://www.w3.org/1999/xhtml";

/// Serialize a document back to polyglot HTML/XHTML: void elements are
/// self-closed, all other elements get explicit end tags, and the original
/// XML declaration (if any) is restored.
pub fn serialize(doc: &Document) -> String {
    let mut out = String::new();
    if let Some(decl) = &doc.xml_declaration {
        out.push_str(decl);
        out.push('\n');
    }
    write_node(&doc.dom.document, false, &mut out);
    out
}

fn write_node(handle: &Handle, raw_text: bool, out: &mut String) {
    match &handle.data {
        NodeData::Document => {
            for child in handle.children.borrow().iter() {
                write_node(child, false, out);
            }
        }
        NodeData::Doctype {
            name,
            public_id,
            system_id,
        } => {
            out.push_str("<!DOCTYPE ");
            out.push_str(name);
            if !public_id.is_empty() {
                out.push_str(&format!(" PUBLIC \"{public_id}\" \"{system_id}\""));
            } else if !system_id.is_empty() {
                out.push_str(&format!(" SYSTEM \"{system_id}\""));
            }
            out.push('>');
        }
        NodeData::Text { contents } => {
            let text = contents.borrow();
            if raw_text {
                out.push_str(&text);
            } else {
                escape_text(&text, out);
            }
        }
        NodeData::Comment { contents } => {
            out.push_str("<!--");
            out.push_str(contents);
            out.push_str("-->");
        }
        NodeData::ProcessingInstruction { target, contents } => {
            out.push_str("<?");
            out.push_str(target);
            out.push(' ');
            out.push_str(contents);
            out.push_str("?>");
        }
        NodeData::Element { name, attrs, .. } => {
            let tag = name.local.as_ref();
            out.push('<');
            out.push_str(tag);

            let attrs = attrs.borrow();
            let mut has_xmlns = false;
            for attr in attrs.iter() {
                let attr_name = qualified_attr_name(attr);
                if attr_name == "xmlns" {
                    has_xmlns = true;
                }
                out.push(' ');
                out.push_str(&attr_name);
                out.push_str("=\"");
                escape_attr(&attr.value, out);
                out.push('"');
            }
            if tag == "html" && !has_xmlns {
                out.push_str(&format!(" xmlns=\"{XHTML_NS}\""));
            }

            let children = handle.children.borrow();
            if children.is_empty() && VOID.contains(&tag) {
                out.push_str("/>");
            } else {
                out.push('>');
                let child_raw = RAW_TEXT.contains(&tag);
                for child in children.iter() {
                    write_node(child, child_raw, out);
                }
                out.push_str("</");
                out.push_str(tag);
                out.push('>');
            }
        }
    }
}

/// Reconstruct an attribute name, including any namespace prefix.
fn qualified_attr_name(attr: &Attribute) -> String {
    let local = attr.name.local.as_ref();
    match &attr.name.prefix {
        Some(prefix) => format!("{}:{}", prefix.as_ref(), local),
        None => local.to_string(),
    }
}

fn escape_text(text: &str, out: &mut String) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\u{00A0}' => out.push_str("&#160;"),
            c => out.push(c),
        }
    }
}

fn escape_attr(value: &str, out: &mut String) {
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '"' => out.push_str("&quot;"),
            '\u{00A0}' => out.push_str("&#160;"),
            c => out.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::parse::parse;
    use super::*;

    #[test]
    fn self_closes_void_elements_only() {
        let doc = parse(b"<html><body><p>hi<br>there</p><div></div></body></html>");
        let out = serialize(&doc);
        assert!(out.contains("<br/>"), "{out}");
        assert!(out.contains("<div></div>"), "{out}");
        assert!(out.contains("<p>hi<br/>there</p>"), "{out}");
    }

    #[test]
    fn escapes_text_and_keeps_script_raw() {
        let doc = parse(b"<html><body><p>a &amp; b &lt; c</p><script>1 < 2 && 3</script></body></html>");
        let out = serialize(&doc);
        assert!(out.contains("a &amp; b &lt; c"), "{out}");
        assert!(out.contains("<script>1 < 2 && 3</script>"), "{out}");
    }

    #[test]
    fn restores_xml_declaration_and_xmlns() {
        let src = b"<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<html><body><p>x</p></body></html>";
        let doc = parse(src);
        let out = serialize(&doc);
        assert!(out.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>"), "{out}");
        assert!(out.contains("xmlns=\"http://www.w3.org/1999/xhtml\""), "{out}");
    }
}
