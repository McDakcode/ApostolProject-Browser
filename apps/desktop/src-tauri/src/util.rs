// Made by MrDuck
#![allow(unused_imports)]

// APB shared low-level helpers: percent/base64 codecs, image magic sniffing.

pub(crate) fn sniff_image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("image/png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if bytes.len() > 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else if bytes.starts_with(b"BM") {
        Some("image/bmp")
    } else if looks_like_svg(bytes) {
        Some("image/svg+xml")
    } else {
        None
    }
}

/// SVG — это текст, не magic bytes: файл может начинаться с `<?xml…?>`,
/// BOM или сразу `<svg`. Ищем тег <svg> в первых 1024 байтах
/// (регистронезависимо) — без этого заметки с SVG-картинками
/// отбраковывались как «не изображение» и рисовались красной рамкой.
fn looks_like_svg(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(1024)];
    let s = String::from_utf8_lossy(head).to_lowercase();
    if s.contains("<svg") { return true; }
    // CSV-обманки/обычный текст с <svg внутри — нет: требуем <svg близко к началу
    s.find('<').map(|i| s[i..].starts_with("<?xml") && s.contains("<svg")).unwrap_or(false)
}

/// Percent-decode `%XX` sequences into a UTF-8 string (inverse of
/// [`percent_encode`]).
pub(crate) fn percent_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 3 <= bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Minimal standard-alphabet base64 encoder (no line wrapping).
pub(crate) fn encode_base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

/// Percent-encode everything outside the RFC 3986 unreserved set.
pub(crate) fn percent_encode(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len());
    for &b in input {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// Minimal standard-alphabet base64 decoder (tolerates data-URL prefixes,
/// whitespace and padding).
pub(crate) fn decode_base64(input: &str) -> Option<Vec<u8>> {
    fn val(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a') as u32 + 26),
            b'0'..=b'9' => Some((c - b'0') as u32 + 52),
            b'+' | b'-' => Some(62),
            b'/' | b'_' => Some(63),
            _ => None,
        }
    }
    let cleaned: Vec<u8> = input
        .rsplit_once("base64,")
        .map(|(_, rest)| rest)
        .unwrap_or(input)
        .bytes()
        .filter(|b| val(*b).is_some())
        .collect();
    let mut out = Vec::with_capacity(cleaned.len() / 4 * 3);
    for chunk in cleaned.chunks(4) {
        if chunk.len() < 2 {
            break;
        }
        let mut acc: u32 = 0;
        for (i, &c) in chunk.iter().enumerate() {
            acc |= val(c)? << (18 - 6 * i);
        }
        out.push((acc >> 16) as u8);
        if chunk.len() > 2 {
            out.push((acc >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(acc as u8);
        }
    }
    Some(out)
}

// ---------------------------------------------------------------------
// Command palette (§22)
// ---------------------------------------------------------------------

// Made by MrDuck
#[cfg(test)]
mod svg_sniff_tests {
    use super::*;

    #[test]
    fn svg_plain_head() {
        assert_eq!(sniff_image_mime(b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>"), Some("image/svg+xml"));
    }
    #[test]
    fn svg_with_xml_prolog() {
        assert_eq!(sniff_image_mime(b"<?xml version=\"1.0\"?>\n<svg viewBox=\"0 0 1 1\">"), Some("image/svg+xml"));
    }
    #[test]
    fn svg_case_insensitive() {
        assert_eq!(sniff_image_mime(b"<SVG width=\"10\">"), Some("image/svg+xml"));
    }
    #[test]
    fn plain_text_not_svg() {
        assert_eq!(sniff_image_mime(b"hello world, this is a text file"), None);
        assert_eq!(sniff_image_mime(b"<html><body>no svg here</body></html>"), None);
    }
}
