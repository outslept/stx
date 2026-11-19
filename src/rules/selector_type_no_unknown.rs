use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::{Span, ast::ComplexSelector};

pub struct SelectorTypeNoUnknown;

const KNOWN_TAGS: &[&str] = &[
    "a",
    "abbr",
    "address",
    "area",
    "article",
    "aside",
    "audio",
    "b",
    "base",
    "bdi",
    "bdo",
    "blockquote",
    "body",
    "br",
    "button",
    "canvas",
    "caption",
    "cite",
    "code",
    "col",
    "colgroup",
    "data",
    "datalist",
    "dd",
    "del",
    "details",
    "dfn",
    "dialog",
    "div",
    "dl",
    "dt",
    "em",
    "embed",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hgroup",
    "hr",
    "html",
    "i",
    "iframe",
    "img",
    "input",
    "ins",
    "kbd",
    "label",
    "legend",
    "li",
    "link",
    "main",
    "map",
    "mark",
    "math",
    "menu",
    "meta",
    "meter",
    "nav",
    "noscript",
    "object",
    "ol",
    "optgroup",
    "option",
    "output",
    "p",
    "param",
    "picture",
    "portal",
    "pre",
    "progress",
    "q",
    "rp",
    "rt",
    "ruby",
    "s",
    "samp",
    "script",
    "search",
    "section",
    "select",
    "slot",
    "small",
    "source",
    "span",
    "strong",
    "style",
    "sub",
    "summary",
    "sup",
    "svg",
    "table",
    "tbody",
    "td",
    "template",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "time",
    "title",
    "tr",
    "track",
    "u",
    "ul",
    "var",
    "video",
    "wbr",
    "animate",
    "animatemotion",
    "animatetransform",
    "circle",
    "clippath",
    "defs",
    "desc",
    "ellipse",
    "feblend",
    "fecolormatrix",
    "fecomponenttransfer",
    "fecomposite",
    "feconvolvematrix",
    "fediffuselighting",
    "fedisplacementmap",
    "fedistantlight",
    "fedropshadow",
    "feflood",
    "fefunca",
    "fefuncb",
    "fefuncg",
    "fefuncr",
    "fegaussianblur",
    "feimage",
    "femerge",
    "femergenode",
    "femorphology",
    "feoffset",
    "fepointlight",
    "fespecularlighting",
    "fespotlight",
    "fetile",
    "feturbulence",
    "filter",
    "foreignobject",
    "g",
    "image",
    "line",
    "lineargradient",
    "marker",
    "mask",
    "metadata",
    "mpath",
    "path",
    "pattern",
    "polygon",
    "polyline",
    "radialgradient",
    "rect",
    "set",
    "stop",
    "switch",
    "symbol",
    "text",
    "textpath",
    "tspan",
    "use",
    "view",
    "annotation",
    "annotation-xml",
    "maction",
    "merror",
    "mfrac",
    "mi",
    "mmultiscripts",
    "mn",
    "mo",
    "mover",
    "mpadded",
    "mphantom",
    "mprescripts",
    "mroot",
    "mrow",
    "ms",
    "mspace",
    "msqrt",
    "mstyle",
    "msub",
    "msubsup",
    "msup",
    "mtable",
    "mtd",
    "mtext",
    "mtr",
    "munder",
    "munderover",
    "semantics",
];

impl Rule for SelectorTypeNoUnknown {
    fn name(&self) -> &'static str {
        "selector-type-no-unknown"
    }

    fn check_complex_selector<'a>(&self, sel: &'a ComplexSelector<'a>, ctx: &mut LintContext<'a>) {
        if ctx.in_keyframes() {
            return;
        }

        let start = sel.span.start as usize;
        let end = sel.span.end as usize;
        let src = ctx.source;
        if start >= end || end > src.len() {
            return;
        }

        let bytes = src.as_bytes();
        let mut i = start;
        let mut at_compound_start = true;

        while i < end {
            if bytes[i].is_ascii_whitespace() || matches!(bytes[i], b'>' | b'+' | b'~' | b',') {
                at_compound_start = true;
                i += 1;
                continue;
            }

            if bytes[i] == b'[' {
                at_compound_start = false;
                i += 1;
                while i < end && bytes[i] != b']' {
                    if bytes[i] == b'"' || bytes[i] == b'\'' {
                        let q = bytes[i];
                        i += 1;
                        while i < end && bytes[i] != q {
                            if bytes[i] == b'\\' {
                                i += 2;
                                continue;
                            }
                            i += 1;
                        }
                    }
                    i += 1;
                }
                i += 1;
                continue;
            }

            if bytes[i] == b'(' {
                at_compound_start = false;
                let mut depth = 1usize;
                i += 1;
                while i < end && depth > 0 {
                    if bytes[i] == b'(' {
                        depth += 1;
                    } else if bytes[i] == b')' {
                        depth -= 1;
                    }
                    i += 1;
                }
                continue;
            }

            if matches!(bytes[i], b'.' | b'#' | b':' | b'%' | b'&' | b'*') {
                at_compound_start = false;
                i += 1;
                while i < end
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
                {
                    i += 1;
                }
                continue;
            }

            if at_compound_start && bytes[i].is_ascii_alphabetic() {
                let tag_start = i;
                while i < end
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
                {
                    i += 1;
                }
                if i < end && bytes[i] == b'|' {
                    i += 1;
                    continue;
                }
                let tag = &src[tag_start..i];
                if !tag.contains('-') {
                    let lower = tag.to_ascii_lowercase();
                    if !KNOWN_TAGS.contains(&lower.as_str()) {
                        ctx.report(
                            self.name(),
                            Span {
                                start: tag_start as u32,
                                end: i as u32,
                            },
                            format!("Unexpected unknown type selector \"{tag}\""),
                        );
                    }
                }
                at_compound_start = false;
                continue;
            }

            at_compound_start = false;
            i += 1;
        }
    }
}
