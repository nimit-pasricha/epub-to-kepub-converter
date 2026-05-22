use std::cell::RefCell;

use markup5ever::tendril::StrTendril;
use markup5ever::{Attribute, LocalName, Namespace, QualName};
use markup5ever_rcdom::{Handle, Node, NodeData};

pub const XHTML_NS: &str = "http://www.w3.org/1999/xhtml";

/// Build a new element node in the XHTML namespace.
pub fn element(tag: &str, attrs: &[(&str, &str)]) -> Handle {
    let attrs = attrs
        .iter()
        .map(|(k, v)| Attribute {
            name: QualName::new(None, Namespace::from(""), LocalName::from(*k)),
            value: StrTendril::from(*v),
        })
        .collect();
    Node::new(NodeData::Element {
        name: QualName::new(None, Namespace::from(XHTML_NS), LocalName::from(tag)),
        attrs: RefCell::new(attrs),
        template_contents: RefCell::new(None),
        mathml_annotation_xml_integration_point: false,
    })
}

/// Build a new text node.
pub fn text(contents: &str) -> Handle {
    Node::new(NodeData::Text {
        contents: RefCell::new(StrTendril::from(contents)),
    })
}

/// The lowercase tag name of an element node, or `None` for other node kinds.
pub fn local_name(handle: &Handle) -> Option<&str> {
    match &handle.data {
        NodeData::Element { name, .. } => Some(name.local.as_ref()),
        _ => None,
    }
}

/// Value of an element attribute.
pub fn get_attr(handle: &Handle, key: &str) -> Option<String> {
    if let NodeData::Element { attrs, .. } = &handle.data {
        for attr in attrs.borrow().iter() {
            if attr.name.local.as_ref() == key {
                return Some(attr.value.to_string());
            }
        }
    }
    None
}

/// The first direct child element with the given tag name.
pub fn find_child(parent: &Handle, tag: &str) -> Option<Handle> {
    parent
        .children
        .borrow()
        .iter()
        .find(|c| local_name(c) == Some(tag))
        .cloned()
}
