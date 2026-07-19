use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SectionSnapshot {
    pub stable_section_key: String,
    pub section_type: String,
    pub section_number: Option<String>,
    pub title: String,
    pub text_content: String,
    pub text_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SectionChange {
    pub stable_section_key: String,
    pub change_type: SectionChangeType,
    pub old_hash: Option<String>,
    pub new_hash: Option<String>,
    pub old_title: Option<String>,
    pub new_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SectionChangeType {
    Added,
    Removed,
    Modified,
    Unchanged,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VersionDiffResult {
    pub added: usize,
    pub removed: usize,
    pub modified: usize,
    pub unchanged: usize,
    pub changes: Vec<SectionChange>,
}

pub fn diff_sections(
    old_sections: &[SectionSnapshot],
    new_sections: &[SectionSnapshot],
) -> VersionDiffResult {
    let old_map: BTreeMap<String, &SectionSnapshot> = old_sections
        .iter()
        .map(|section| (section.stable_section_key.clone(), section))
        .collect();

    let new_map: BTreeMap<String, &SectionSnapshot> = new_sections
        .iter()
        .map(|section| (section.stable_section_key.clone(), section))
        .collect();

    let mut keys = BTreeSet::new();
    keys.extend(old_map.keys().cloned());
    keys.extend(new_map.keys().cloned());

    let mut result = VersionDiffResult {
        added: 0,
        removed: 0,
        modified: 0,
        unchanged: 0,
        changes: Vec::new(),
    };

    for key in keys {
        match (old_map.get(&key), new_map.get(&key)) {
            (None, Some(new_section)) => {
                result.added += 1;
                result.changes.push(SectionChange {
                    stable_section_key: key,
                    change_type: SectionChangeType::Added,
                    old_hash: None,
                    new_hash: Some(new_section.text_hash.clone()),
                    old_title: None,
                    new_title: Some(new_section.title.clone()),
                });
            }
            (Some(old_section), None) => {
                result.removed += 1;
                result.changes.push(SectionChange {
                    stable_section_key: key,
                    change_type: SectionChangeType::Removed,
                    old_hash: Some(old_section.text_hash.clone()),
                    new_hash: None,
                    old_title: Some(old_section.title.clone()),
                    new_title: None,
                });
            }
            (Some(old_section), Some(new_section))
                if old_section.text_hash != new_section.text_hash =>
            {
                result.modified += 1;
                result.changes.push(SectionChange {
                    stable_section_key: key,
                    change_type: SectionChangeType::Modified,
                    old_hash: Some(old_section.text_hash.clone()),
                    new_hash: Some(new_section.text_hash.clone()),
                    old_title: Some(old_section.title.clone()),
                    new_title: Some(new_section.title.clone()),
                });
            }
            (Some(old_section), Some(new_section)) => {
                result.unchanged += 1;
                result.changes.push(SectionChange {
                    stable_section_key: key,
                    change_type: SectionChangeType::Unchanged,
                    old_hash: Some(old_section.text_hash.clone()),
                    new_hash: Some(new_section.text_hash.clone()),
                    old_title: Some(old_section.title.clone()),
                    new_title: Some(new_section.title.clone()),
                });
            }
            (None, None) => {}
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section(key: &str, hash: &str) -> SectionSnapshot {
        SectionSnapshot {
            stable_section_key: key.to_string(),
            section_type: "article".to_string(),
            section_number: Some(key.to_string()),
            title: format!("Article {key}"),
            text_content: format!("Text {hash}"),
            text_hash: hash.to_string(),
        }
    }

    #[test]
    fn detects_added_removed_modified_and_unchanged_sections() {
        let old_sections = vec![
            section("article-1", "aaa"),
            section("article-2", "bbb"),
            section("article-3", "ccc"),
        ];

        let new_sections = vec![
            section("article-1", "aaa"),
            section("article-2", "changed"),
            section("article-4", "ddd"),
        ];

        let diff = diff_sections(&old_sections, &new_sections);

        assert_eq!(diff.added, 1);
        assert_eq!(diff.removed, 1);
        assert_eq!(diff.modified, 1);
        assert_eq!(diff.unchanged, 1);
    }
}
