use markup5ever_rcdom::{Handle, NodeData};

use super::dom::{element, find_child, get_attr, text};
use super::parse::Document;
use super::sentences::split_sentences;

/// Elements whose contents are emitted untouched (no spans inside).
const SKIP: &[&str] = &["script", "style", "pre", "audio", "video", "svg", "math"];

/// Block-level elements that start a new paragraph counter.
const BLOCK: &[&str] = &["p", "ol", "ul", "table", "h1", "h2", "h3", "h4", "h5", "h6"];

/// Paragraph/segment counters for koboSpan ids.
struct State {
    para: u32,
    seg: u32,
}

impl State {
    fn alloc_id(&mut self) -> String {
        if self.para == 0 {
            self.para = 1;
        }
        format!("kobo.{}.{}", self.para, self.seg)
    }
}

/// Inject `koboSpan` tags around every text fragment and image. Idempotent:
/// a document that already contains koboSpans is left unchanged.
pub fn transform(doc: &Document) {
    let Some(html) = find_child(&doc.dom.document, "html") else {
        return;
    };
    let Some(body) = find_child(&html, "body") else {
        return;
    };
    if has_kobospan(&body) {
        return;
    }
    let mut state = State { para: 0, seg: 0 };
    walk(&body, "body", &mut state);
}

fn has_kobospan(node: &Handle) -> bool {
    let is_span = get_attr(node, "class")
        .map(|c| c.split_whitespace().any(|p| p == "koboSpan"))
        .unwrap_or(false);
    is_span || node.children.borrow().iter().any(has_kobospan)
}

fn walk(node: &Handle, parent_tag: &str, state: &mut State) {
    let new_children: Vec<Handle> = {
        let children = node.children.borrow();
        let mut out = Vec::with_capacity(children.len());
        for child in children.iter() {
            match &child.data {
                NodeData::Text { contents } => {
                    let content = contents.borrow().to_string();
                    process_text(&content, parent_tag, state, &mut out);
                }
                NodeData::Element { name, .. } => {
                    let tag = name.local.as_ref().to_string();
                    if SKIP.contains(&tag.as_str()) {
                        out.push(child.clone());
                    } else if tag == "img" {
                        state.para += 1;
                        state.seg += 1;
                        out.push(span_around(child.clone(), state));
                    } else {
                        if BLOCK.contains(&tag.as_str()) {
                            state.para += 1;
                            state.seg = 0;
                        }
                        walk(child, &tag, state);
                        out.push(child.clone());
                    }
                }
                _ => out.push(child.clone()),
            }
        }
        out
    };
    *node.children.borrow_mut() = new_children;
}

fn process_text(content: &str, parent_tag: &str, state: &mut State, out: &mut Vec<Handle>) {
    if content.is_empty() {
        return;
    }
    if content.chars().all(char::is_whitespace) {
        // Whitespace-only nodes are wrapped only inside paragraphs, so the
        // paragraph's spans stay contiguous; elsewhere they pass through.
        if parent_tag == "p" {
            state.seg += 1;
            out.push(span_with_text(content, state));
        } else {
            out.push(text(content));
        }
        return;
    }
    for fragment in split_sentences(content) {
        state.seg += 1;
        out.push(span_with_text(fragment, state));
    }
}

fn span_with_text(content: &str, state: &mut State) -> Handle {
    let span = new_span(state);
    span.children.borrow_mut().push(text(content));
    span
}

fn span_around(child: Handle, state: &mut State) -> Handle {
    let span = new_span(state);
    span.children.borrow_mut().push(child);
    span
}

fn new_span(state: &mut State) -> Handle {
    let id = state.alloc_id();
    element("span", &[("class", "koboSpan"), ("id", &id)])
}

#[cfg(test)]
mod tests {
    use super::super::parse::parse;
    use super::super::serialize::serialize;
    use super::*;

    #[test]
    fn wraps_sentences_in_numbered_spans() {
        let doc = parse(b"<html><body><p>One. Two.</p></body></html>");
        transform(&doc);
        let out = serialize(&doc);
        assert!(out.contains(r#"<span class="koboSpan" id="kobo.1.1">One. </span>"#), "{out}");
        assert!(out.contains(r#"<span class="koboSpan" id="kobo.1.2">Two.</span>"#), "{out}");
    }

    #[test]
    fn separate_paragraphs_get_separate_counters() {
        let doc = parse(b"<html><body><p>A.</p><p>B.</p></body></html>");
        transform(&doc);
        let out = serialize(&doc);
        assert!(out.contains(r#"id="kobo.1.1">A.</span>"#), "{out}");
        assert!(out.contains(r#"id="kobo.2.1">B.</span>"#), "{out}");
    }

    #[test]
    fn wraps_images() {
        let doc = parse(b"<html><body><div><img src=\"c.jpg\"/></div></body></html>");
        transform(&doc);
        let out = serialize(&doc);
        assert!(out.contains(r#"<span class="koboSpan" id="kobo.1.1"><img src="c.jpg"/></span>"#), "{out}");
    }

    #[test]
    fn leaves_pre_untouched() {
        let doc = parse(b"<html><body><pre>code. here.</pre></body></html>");
        transform(&doc);
        let out = serialize(&doc);
        assert!(out.contains("<pre>code. here.</pre>"), "{out}");
        assert!(!out.contains("koboSpan"), "{out}");
    }

    #[test]
    fn is_idempotent() {
        let doc = parse(b"<html><body><p>One. Two.</p></body></html>");
        transform(&doc);
        let once = serialize(&doc);
        transform(&doc);
        assert_eq!(once, serialize(&doc));
    }
}
