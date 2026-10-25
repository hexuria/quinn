//! Pull the `script_mod!` body back out the way Makepad's watcher does.
//!
//! The watcher compares that text to the `ScriptMod.code` we eval'd. The
//! generated file has no comments inside the block, and its only `#(` is the
//! component placeholder, so this matches `normalize_script_mod_body`.

#[derive(Clone, Debug, PartialEq)]
pub struct Extracted {
    pub line: usize,
    pub column: usize,
    pub code: String,
}

pub fn extract_hot_script(source: &str) -> Result<Extracted, String> {
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'/') {
            i = skip_line(bytes, i);
            continue;
        }
        if bytes[i] == b'"' {
            i = skip_string(bytes, i)?;
            continue;
        }
        if is_ident_start(bytes[i]) {
            let start = i;
            i += 1;
            while i < bytes.len() && is_ident_continue(bytes[i]) {
                i += 1;
            }
            if &source[start..i] == "script_mod" {
                let mut j = skip_ws(bytes, i);
                if bytes.get(j) == Some(&b'!') {
                    j += 1;
                    j = skip_ws(bytes, j);
                    if bytes.get(j) == Some(&b'{') {
                        let end = find_matching(bytes, j, b'{', b'}')?;
                        let body = &source[j + 1..end];
                        return normalize(source, j, body);
                    }
                }
            }
            continue;
        }
        i += 1;
    }
    Err("hot file has no script_mod! block".into())
}

fn normalize(source: &str, brace: usize, body: &str) -> Result<Extracted, String> {
    let (line, column) = first_token_pos(source, brace, body);
    let bytes = body.as_bytes();
    let mut i = 0;
    let mut out = String::with_capacity(body.len() + 8);
    let mut values = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let end = skip_string(bytes, i)?;
            out.push_str(&body[i..end]);
            i = end;
            continue;
        }
        if bytes[i] == b'#' && bytes.get(i + 1) == Some(&b'(') {
            let end = find_matching(bytes, i + 1, b'(', b')')? + 1;
            out.push_str(&format!("#({values})"));
            values += 1;
            i = end;
            continue;
        }
        out.push(body[i..].chars().next().unwrap());
        i += body[i..].chars().next().unwrap().len_utf8();
    }
    out.push(';');
    Ok(Extracted { line, column, code: out })
}

fn first_token_pos(source: &str, brace: usize, body: &str) -> (usize, usize) {
    let mut line = 1usize;
    let mut column = 1usize;
    for byte in source.as_bytes().iter().take(brace + 1) {
        if *byte == b'\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    for ch in body.chars() {
        if ch == '\n' {
            line += 1;
            column = 1;
            continue;
        }
        if ch.is_whitespace() {
            column += 1;
            continue;
        }
        return (line, column);
    }
    (line, column)
}

fn skip_ws(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

fn skip_line(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
    }
    i
}

fn skip_string(bytes: &[u8], mut i: usize) -> Result<usize, String> {
    i += 1;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'"' {
            return Ok(i + 1);
        }
        i += 1;
    }
    Err("unclosed string in hot file".into())
}

fn find_matching(bytes: &[u8], open_at: usize, open: u8, close: u8) -> Result<usize, String> {
    let mut depth = 0usize;
    let mut i = open_at;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            i = skip_string(bytes, i)?;
            continue;
        }
        if bytes[i] == open {
            depth += 1;
        } else if bytes[i] == close {
            depth -= 1;
            if depth == 0 {
                return Ok(i);
            }
        }
        i += 1;
    }
    Err("unclosed delimiter in hot file".into())
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_ident_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}
