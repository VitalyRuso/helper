use sha2::{Digest, Sha256};

use super::diff::SectionSnapshot;

pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn simple_fixture_sections(text: &str) -> Vec<SectionSnapshot> {
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                return None;
            }

            let stable_section_key = format!("section-{}", index + 1);
            let text_hash = sha256_hex(trimmed);

            Some(SectionSnapshot {
                stable_section_key,
                section_type: "paragraph".to_string(),
                section_number: Some((index + 1).to_string()),
                title: format!("Section {}", index + 1),
                text_content: trimmed.to_string(),
                text_hash,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_deterministic_fixture_sections() {
        let first = simple_fixture_sections("Uno\nDos");
        let second = simple_fixture_sections("Uno\nDos");

        assert_eq!(first.len(), 2);
        assert_eq!(first[0].text_hash, second[0].text_hash);
    }
}
