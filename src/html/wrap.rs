use markup5ever_rcdom::Handle;

use super::dom::{element, find_child, get_attr, local_name, text};
use super::parse::Document;

const KOBO_STYLE: &str = "div#book-inner { margin-top: 0; margin-bottom: 0;}";

/// Apply the Kobo structural wrapping to a content document: wrap the body in
/// `div#book-columns > div#book-inner` and add the kobo style hack to the head.
/// Both steps are idempotent.
pub fn transform(doc: &Document) {
    let Some(html) = find_child(&doc.dom.document, "html") else {
        return;
    };
    if let Some(head) = find_child(&html, "head") {
        ensure_style(&head);
    }
    if let Some(body) = find_child(&html, "body") {
        wrap_body(&body);
    }
}

fn wrap_body(body: &Handle) {
    let already_wrapped = body.children.borrow().iter().any(|c| {
        local_name(c) == Some("div") && get_attr(c, "id").as_deref() == Some("book-columns")
    });
    if already_wrapped {
        return;
    }

    let original = std::mem::take(&mut *body.children.borrow_mut());
    let inner = element("div", &[("id", "book-inner")]);
    *inner.children.borrow_mut() = original;
    let columns = element("div", &[("id", "book-columns")]);
    columns.children.borrow_mut().push(inner);
    body.children.borrow_mut().push(columns);
}

fn ensure_style(head: &Handle) {
    let exists = head.children.borrow().iter().any(|c| {
        local_name(c) == Some("style") && get_attr(c, "id").as_deref() == Some("kobostylehacks")
    });
    if exists {
        return;
    }

    let style = element("style", &[("id", "kobostylehacks"), ("type", "text/css")]);
    style.children.borrow_mut().push(text(KOBO_STYLE));
    head.children.borrow_mut().push(style);
}

#[cfg(test)]
mod tests {
    use super::super::parse::parse;
    use super::super::serialize::serialize;
    use super::*;

    #[test]
    fn wraps_body_and_adds_style() {
        let doc = parse(b"<html><head></head><body><p>hi</p></body></html>");
        transform(&doc);
        let out = serialize(&doc);
        assert!(out.contains(r#"<div id="book-columns"><div id="book-inner"><p>hi</p>"#), "{out}");
        assert!(out.contains(r#"<style id="kobostylehacks""#), "{out}");
    }

    #[test]
    fn is_idempotent() {
        let doc = parse(b"<html><head></head><body><p>hi</p></body></html>");
        transform(&doc);
        let once = serialize(&doc);
        transform(&doc);
        assert_eq!(once, serialize(&doc));
    }
}
