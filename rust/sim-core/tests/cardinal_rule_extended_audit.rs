//! Extra static audit coverage for cardinal-rule boundaries.
//!
//! These tests intentionally scan production source text. They are not a
//! substitute for behavior tests; they are early tripwires for accidental engine
//! writes that would bypass the intended biological/social pipelines.

use std::fs;
use std::path::{Path, PathBuf};

fn src_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn collect_source_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read src dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            collect_source_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn production_source(path: &Path) -> String {
    let content = fs::read_to_string(path).unwrap_or_default();
    match content.find("#[cfg(test)]") {
        Some(idx) => content[..idx].to_string(),
        None => content,
    }
}

fn file_name(path: &Path) -> &str {
    path.file_name().and_then(|n| n.to_str()).unwrap_or("")
}

#[test]
fn only_genome_rs_assigns_concrete_chromosome_values() {
    let mut files = Vec::new();
    collect_source_files(&src_dir(), &mut files);

    let violators: Vec<String> = files
        .iter()
        .filter(|f| file_name(f) != "genome.rs")
        .filter(|f| {
            let src = production_source(f);
            src.contains("chromosome: Some(") || src.contains(".chromosome = Some(")
        })
        .map(|f| f.display().to_string())
        .collect();

    assert!(
        violators.is_empty(),
        "concrete chromosome values should be assigned only by genome.rs: {violators:?}"
    );
}

#[test]
fn child_creation_wires_parent_ids_from_the_two_parents() {
    let individual_rs = src_dir().join("biology").join("individual.rs");
    let src = production_source(&individual_rs);
    let create_child_start = src.find("pub fn create_child").expect("create_child exists");
    let create_child_src = &src[create_child_start..];

    assert!(
        create_child_src.contains("parent_1_id: Some(parent1.id.clone())"),
        "create_child must wire parent_1_id from parent1"
    );
    assert!(
        create_child_src.contains("parent_2_id: Some(parent2.id.clone())"),
        "create_child must wire parent_2_id from parent2"
    );
}

#[test]
fn engine_code_does_not_seed_child_technology_or_belief_state_in_create_child() {
    let individual_rs = src_dir().join("biology").join("individual.rs");
    let src = production_source(&individual_rs);
    let create_child_start = src.find("pub fn create_child").expect("create_child exists");
    let create_child_src = &src[create_child_start..];

    assert!(
        create_child_src.contains("known_techs: vec![]"),
        "children must not be born with pre-seeded technologies"
    );
    assert!(
        create_child_src.contains("beliefs: Default::default()"),
        "children must not be born with pre-seeded beliefs"
    );
}
