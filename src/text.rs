pub fn without_string_literals(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = bytes.to_vec();
    let mut i = 0;

    while i < bytes.len() {
        if let Some(end) = rust_raw_string_end(bytes, i) {
            blank_range(&mut out, i, end);
            i = end;
            continue;
        }

        if let Some(end) = triple_quote_end(bytes, i) {
            blank_range(&mut out, i, end);
            i = end;
            continue;
        }

        if matches!(bytes[i], b'"' | b'\'' | b'`') {
            let end = quoted_string_end(bytes, i);
            blank_range(&mut out, i, end);
            i = end;
            continue;
        }

        i += 1;
    }

    String::from_utf8(out).unwrap_or_else(|_| text.to_string())
}

fn rust_raw_string_end(bytes: &[u8], start: usize) -> Option<usize> {
    let raw_start = if bytes.get(start) == Some(&b'r') {
        start
    } else if bytes.get(start) == Some(&b'b') && bytes.get(start + 1) == Some(&b'r') {
        start + 1
    } else {
        return None;
    };

    let mut cursor = raw_start + 1;
    let mut hashes = 0;
    while bytes.get(cursor) == Some(&b'#') {
        hashes += 1;
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b'"') {
        return None;
    }

    cursor += 1;
    while cursor < bytes.len() {
        if bytes[cursor] == b'"' && closes_hashes(bytes, cursor + 1, hashes) {
            return Some(cursor + 1 + hashes);
        }
        cursor += 1;
    }
    Some(bytes.len())
}

fn closes_hashes(bytes: &[u8], start: usize, hashes: usize) -> bool {
    (0..hashes).all(|offset| bytes.get(start + offset) == Some(&b'#'))
}

fn triple_quote_end(bytes: &[u8], start: usize) -> Option<usize> {
    let quote = *bytes.get(start)?;
    if !matches!(quote, b'"' | b'\'') {
        return None;
    }
    if bytes.get(start + 1) != Some(&quote) || bytes.get(start + 2) != Some(&quote) {
        return None;
    }

    let mut cursor = start + 3;
    while cursor + 2 < bytes.len() {
        if bytes[cursor] == quote && bytes[cursor + 1] == quote && bytes[cursor + 2] == quote {
            return Some(cursor + 3);
        }
        cursor += 1;
    }
    Some(bytes.len())
}

fn quoted_string_end(bytes: &[u8], start: usize) -> usize {
    let quote = bytes[start];
    let mut escaped = false;
    let mut cursor = start + 1;

    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if escaped {
            escaped = false;
        } else if byte == b'\\' && quote != b'`' {
            escaped = true;
        } else if byte == quote {
            return cursor + 1;
        }
        cursor += 1;
    }

    bytes.len()
}

fn blank_range(out: &mut [u8], start: usize, end: usize) {
    for byte in &mut out[start..end] {
        if !matches!(*byte, b'\n' | b'\r') {
            *byte = b' ';
        }
    }
}

#[cfg(test)]
mod tests {
    use super::without_string_literals;

    #[test]
    fn strips_raw_strings_with_inner_quotes() {
        let stripped = without_string_literals("let s = r#\"alpha \" beta\"#;\ncallBeta();");

        assert!(!stripped.contains("alpha"));
        assert!(!stripped.contains("beta\"#"));
        assert!(stripped.contains("callBeta"));
    }

    #[test]
    fn strips_python_triple_quoted_strings() {
        let stripped = without_string_literals("x = \"\"\"alpha\nbeta\"\"\"\ncall_beta()");

        assert!(!stripped.contains("alpha"));
        assert!(!stripped.contains("beta\"\"\""));
        assert!(stripped.contains("call_beta"));
    }
}
