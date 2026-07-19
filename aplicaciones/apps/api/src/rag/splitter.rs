pub fn split_text(text: &str, size: usize, overlap: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return vec![];
    }
    let overlap = overlap.min(size.saturating_sub(1));
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let end = (start + size).min(chars.len());
        let chunk: String = chars[start..end].iter().collect();
        if !chunk.trim().is_empty() {
            chunks.push(chunk.trim().to_owned());
        }
        if end == chars.len() {
            break;
        }
        start = end - overlap;
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlaps_chunks() {
        let chunks = split_text("abcdefghij", 4, 2);
        assert_eq!(chunks, vec!["abcd", "cdef", "efgh", "ghij"]);
    }
}
