use html5ever::tendril::TendrilSink;
use html5ever::{ParseOpts, parse_document};
use markup5ever_rcdom::{Handle, NodeData, RcDom};

/// Extract visible text from HTML (document or fragment) via html5ever:
/// spec-compliant parsing and full entity decoding; drops <head>/<style>/<script>
/// contents and puts newlines around block-level elements. Good for grep and
/// diffing, not a layout-faithful rendering.
pub fn html_to_text(html: &str) -> String {
    let dom = parse_document(RcDom::default(), ParseOpts::default())
        .from_utf8()
        .one(html.as_bytes());

    let mut out = String::with_capacity(html.len());
    walk(&dom.document, &mut out);

    let mut squeezed = String::with_capacity(out.len());
    let mut newlines = 0;
    for c in out.chars() {
        if c == '\n' {
            newlines += 1;
            if newlines <= 2 {
                squeezed.push(c);
            }
        } else {
            newlines = 0;
            squeezed.push(c);
        }
    }
    squeezed.trim().to_string()
}

fn walk(node: &Handle, out: &mut String) {
    match &node.data {
        NodeData::Text { contents } => out.push_str(&contents.borrow()),
        NodeData::Element { name, .. } => {
            let tag: &str = &name.local;
            if matches!(tag, "head" | "script" | "style") {
                return;
            }
            let block = is_block(tag);
            if block {
                out.push('\n');
            }
            for child in node.children.borrow().iter() {
                walk(child, out);
            }
            if tag == "br" || block {
                out.push('\n');
            }
        }
        _ => {
            for child in node.children.borrow().iter() {
                walk(child, out);
            }
        }
    }
}

fn is_block(tag: &str) -> bool {
    matches!(
        tag,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "caption"
            | "center"
            | "details"
            | "div"
            | "dd"
            | "dl"
            | "dt"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hr"
            | "li"
            | "main"
            | "nav"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "table"
            | "tbody"
            | "tfoot"
            | "thead"
            | "tr"
            | "ul"
    )
}
