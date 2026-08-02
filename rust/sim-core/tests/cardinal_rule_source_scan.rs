//! Static source-scan regression tests, ported from the JS suite's
//! cardinal-rule.test.js. These don't verify runtime behavior (see
//! cardinal-rule-behavioral coverage inside the library crate for that) --
//! they lock down *which files* are allowed to write certain fields at all,
//! catching a violation the moment it's typed, before it ever runs.

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

/// Everything from the first `#[cfg(test)]` onward is test-fixture setup, not
/// production engine code -- cardinal-rule writes are only forbidden there.
fn production_source(path: &Path) -> String {
    let content = fs::read_to_string(path).unwrap_or_default();
    match content.find("#[cfg(test)]") {
        Some(idx) => content[..idx].to_string(),
        None => content,
    }
}

/// True if `field` appears in `src` immediately followed by a single `=`
/// (a direct assignment), as opposed to `==`, `>=`, `<=`, or `!=`.
fn assigns_field(src: &str, field: &str) -> bool {
    let mut search_from = 0;
    while let Some(pos) = src[search_from..].find(field) {
        let after = search_from + pos + field.len();
        let rest = src[after..].trim_start();
        if let Some(rest) = rest.strip_prefix('=') {
            if !rest.starts_with('=') {
                return true;
            }
        }
        search_from = after;
    }
    false
}

fn file_name(path: &Path) -> &str {
    path.file_name().and_then(|n| n.to_str()).unwrap_or("")
}

#[test]
fn only_consciousness_rs_may_directly_assign_mind_consciousness() {
    let mut files = Vec::new();
    collect_source_files(&src_dir(), &mut files);
    let violators: Vec<String> = files
        .iter()
        .filter(|f| file_name(f) != "consciousness.rs")
        .filter(|f| assigns_field(&production_source(f), ".mind.consciousness"))
        .map(|f| f.display().to_string())
        .collect();
    assert!(violators.is_empty(), "files writing .mind.consciousness outside consciousness.rs: {violators:?}");
}

#[test]
fn foxp2_expression_is_never_directly_assigned_outside_language_rs() {
    let mut files = Vec::new();
    collect_source_files(&src_dir(), &mut files);
    let allowed = ["language.rs", "individual.rs"];
    let violators: Vec<String> = files
        .iter()
        .filter(|f| !allowed.contains(&file_name(f)))
        .filter(|f| assigns_field(&production_source(f), ".foxp2_expression"))
        .map(|f| f.display().to_string())
        .collect();
    assert!(violators.is_empty(), "foxp2_expression written outside allowed files: {violators:?}");
}

#[test]
fn beliefs_are_only_inserted_via_belief_rs() {
    let mut files = Vec::new();
    collect_source_files(&src_dir(), &mut files);
    let violators: Vec<String> = files
        .iter()
        .filter(|f| file_name(f) != "belief.rs")
        .filter(|f| production_source(f).contains(".beliefs.insert("))
        .map(|f| f.display().to_string())
        .collect();
    assert!(violators.is_empty(), ".beliefs.insert() called outside belief.rs: {violators:?}");
}

#[test]
fn no_engine_sets_max_lifespan_directly() {
    // max_lifespan is only ever computed fresh by genome::compute_phenotype (a struct
    // literal, not an assignment) or adjusted through the founder-gated god-mode
    // longevity intervention in sim-server (outside sim-core's scope entirely).
    let mut files = Vec::new();
    collect_source_files(&src_dir(), &mut files);
    let violators: Vec<String> = files
        .iter()
        .filter(|f| assigns_field(&production_source(f), ".max_lifespan"))
        .map(|f| f.display().to_string())
        .collect();
    assert!(violators.is_empty(), "engines writing .max_lifespan directly: {violators:?}");
}

#[test]
fn only_hormones_rs_may_directly_assign_individual_hormones() {
    // biology/individual.rs's struct-literal `hormones: Default::default()`
    // placeholders don't match this pattern (no leading `.`) -- they're
    // immediately overwritten by hormones::initialize_hormones right after
    // construction, which does match and is the allowed writer.
    let mut files = Vec::new();
    collect_source_files(&src_dir(), &mut files);
    let violators: Vec<String> = files
        .iter()
        .filter(|f| file_name(f) != "hormones.rs")
        .filter(|f| assigns_field(&production_source(f), ".hormones"))
        .map(|f| f.display().to_string())
        .collect();
    assert!(violators.is_empty(), "files writing .hormones outside hormones.rs: {violators:?}");
}

#[test]
fn known_techs_are_only_added_via_technology_rs_or_agent_rs() {
    let mut files = Vec::new();
    collect_source_files(&src_dir(), &mut files);
    let allowed = ["technology.rs", "agent.rs"];
    let violators: Vec<String> = files
        .iter()
        .filter(|f| !allowed.contains(&file_name(f)))
        .filter(|f| production_source(f).contains(".known_techs.push("))
        .map(|f| f.display().to_string())
        .collect();
    assert!(violators.is_empty(), ".known_techs.push() called outside allowed files: {violators:?}");
}
