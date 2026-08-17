use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Style,
    AtBlock { name: String, params: String },
    AtStatement { name: String },
}

#[derive(Debug, Clone)]
pub struct SourceNode {
    pub kind: NodeKind,
    pub start: usize,
    pub end: usize,
    pub prelude_range: Range<usize>,
    pub body_range: Option<Range<usize>>,
}

impl SourceNode {
    pub fn prelude<'a>(&self, source: &'a str) -> &'a str {
        source[self.prelude_range.clone()].trim()
    }

    pub fn body<'a>(&self, source: &'a str) -> Option<&'a str> {
        self.body_range.as_ref().map(|range| &source[range.clone()])
    }
}

pub fn scan_nodes(source: &str, range: Range<usize>) -> Vec<SourceNode> {
    let bytes = source.as_bytes();
    let mut nodes = Vec::new();
    let mut i = range.start;

    while i < range.end {
        i = skip_trivia(source, i, range.end);
        if i >= range.end || bytes[i] == b'}' {
            break;
        }

        let start = i;
        let mut quote: Option<u8> = None;
        let mut escaped = false;
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut consumed = false;

        while i < range.end {
            let b = bytes[i];

            if let Some(q) = quote {
                if escaped {
                    escaped = false;
                } else if b == b'\\' {
                    escaped = true;
                } else if b == q {
                    quote = None;
                }
                i += 1;
                continue;
            }

            if b == b'/' && i + 1 < range.end && bytes[i + 1] == b'*' {
                i = skip_comment(source, i, range.end);
                continue;
            }

            match b {
                b'\'' | b'"' => {
                    quote = Some(b);
                    i += 1;
                }
                b'(' => {
                    paren_depth += 1;
                    i += 1;
                }
                b')' => {
                    paren_depth = paren_depth.saturating_sub(1);
                    i += 1;
                }
                b'[' => {
                    bracket_depth += 1;
                    i += 1;
                }
                b']' => {
                    bracket_depth = bracket_depth.saturating_sub(1);
                    i += 1;
                }
                b'{' if paren_depth == 0 && bracket_depth == 0 => {
                    let open = i;
                    if let Some(close) = find_matching_brace(source, open, range.end) {
                        let prelude = &source[start..open];
                        let (kind, prelude_start) = classify_block(prelude, start);
                        nodes.push(SourceNode {
                            kind,
                            start,
                            end: close + 1,
                            prelude_range: prelude_start..open,
                            body_range: Some(open + 1..close),
                        });
                        i = close + 1;
                    } else {
                        i = range.end;
                    }
                    consumed = true;
                    break;
                }
                b';' if paren_depth == 0 && bracket_depth == 0 => {
                    let raw = source[start..i].trim();
                    if raw.starts_with('@') {
                        let name = at_rule_name(raw);
                        nodes.push(SourceNode {
                            kind: NodeKind::AtStatement { name },
                            start,
                            end: i + 1,
                            prelude_range: start..i,
                            body_range: None,
                        });
                    }
                    i += 1;
                    consumed = true;
                    break;
                }
                b'}' if paren_depth == 0 && bracket_depth == 0 => {
                    consumed = true;
                    break;
                }
                _ => i += 1,
            }
        }

        if !consumed {
            break;
        }
    }

    nodes
}

fn classify_block(prelude: &str, absolute_start: usize) -> (NodeKind, usize) {
    let leading = prelude.len() - prelude.trim_start().len();
    let trimmed = prelude.trim();
    let prelude_start = absolute_start + leading;
    if trimmed.starts_with('@') {
        let name = at_rule_name(trimmed);
        let after_name = &trimmed[1 + name.len()..];
        let params = after_name.trim().to_string();
        (NodeKind::AtBlock { name, params }, prelude_start)
    } else {
        (NodeKind::Style, prelude_start)
    }
}

fn at_rule_name(raw: &str) -> String {
    raw.trim_start_matches('@')
        .split(|c: char| c.is_ascii_whitespace() || c == '(' || c == ';' || c == '{')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
}

pub fn skip_trivia(source: &str, mut i: usize, end: usize) -> usize {
    let bytes = source.as_bytes();
    loop {
        while i < end && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i + 1 < end && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i = skip_comment(source, i, end);
            continue;
        }
        return i;
    }
}

pub fn is_whitespace_only(source: &str, range: Range<usize>) -> bool {
    source[range].bytes().all(|b| b.is_ascii_whitespace())
}

fn skip_comment(source: &str, i: usize, end: usize) -> usize {
    let bytes = source.as_bytes();
    let mut p = i + 2;
    while p + 1 < end {
        if bytes[p] == b'*' && bytes[p + 1] == b'/' {
            return p + 2;
        }
        p += 1;
    }
    end
}

fn find_matching_brace(source: &str, open: usize, end: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut depth = 1usize;
    let mut i = open + 1;
    let mut quote: Option<u8> = None;
    let mut escaped = false;

    while i < end {
        let b = bytes[i];

        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == q {
                quote = None;
            }
            i += 1;
            continue;
        }

        if b == b'/' && i + 1 < end && bytes[i + 1] == b'*' {
            i = skip_comment(source, i, end);
            continue;
        }

        match b {
            b'\'' | b'"' => quote = Some(b),
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }

    None
}

pub fn count_top_level_declarations(body: &str) -> usize {
    let bytes = body.as_bytes();
    let mut i = 0usize;
    let mut braces = 0usize;
    let mut parens = 0usize;
    let mut brackets = 0usize;
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    let mut count = 0usize;
    let mut segment_has_colon = false;

    while i < bytes.len() {
        let b = bytes[i];
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i = skip_comment(body, i, bytes.len());
            continue;
        }
        match b {
            b'\'' | b'"' => quote = Some(b),
            b'{' => braces += 1,
            b'}' => braces = braces.saturating_sub(1),
            b'(' => parens += 1,
            b')' => parens = parens.saturating_sub(1),
            b'[' => brackets += 1,
            b']' => brackets = brackets.saturating_sub(1),
            b':' if braces == 0 && parens == 0 && brackets == 0 => segment_has_colon = true,
            b';' if braces == 0 && parens == 0 && brackets == 0 => {
                if segment_has_colon {
                    count += 1;
                }
                segment_has_colon = false;
            }
            _ => {}
        }
        i += 1;
    }
    if segment_has_colon {
        count += 1;
    }
    count
}

pub fn count_ascii_case_insensitive_outside_comments(source: &str, needle: &str) -> usize {
    let lower_needle = needle.to_ascii_lowercase();
    let bytes = source.as_bytes();
    let mut i = 0usize;
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    let mut count = 0usize;

    while i < bytes.len() {
        let b = bytes[i];
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i = skip_comment(source, i, bytes.len());
            continue;
        }
        if b == b'\'' || b == b'"' {
            quote = Some(b);
            i += 1;
            continue;
        }
        if i + lower_needle.len() <= bytes.len() {
            let needle = lower_needle.as_bytes();
            let matches = bytes[i..i + needle.len()]
                .iter()
                .zip(needle.iter())
                .all(|(a, b)| a.to_ascii_lowercase() == *b);
            if matches {
                count += 1;
                i += needle.len();
                continue;
            }
        }
        i += 1;
    }
    count
}
