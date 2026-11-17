//! The worked example in `examples/profile/`: a person, their family and their organisation,
//! three cells that link to each other by `@id`. Every cell must validate with no errors or
//! warnings, and every `@id` one cell points at must be another cell in the set.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;
use siafudb_cell::{parse, validate};

fn cells() -> Vec<(String, Value)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/profile");
    let mut out: Vec<(String, Value)> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "cell"))
        .map(|p| {
            let text = std::fs::read_to_string(&p).unwrap();
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            (name, parse(&text).unwrap())
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(out.len(), 3, "person, family and organisation");
    out
}

/// Every `{"@id": ...}` reference inside a value, not counting its own top-level `@id`.
fn links(v: &Value, top: bool, out: &mut BTreeSet<String>) {
    match v {
        Value::Object(m) => {
            if !top {
                if let Some(Value::String(id)) = m.get("@id") {
                    out.insert(id.clone());
                }
            }
            for (k, x) in m {
                if !(top && k == "@id") {
                    links(x, false, out);
                }
            }
        }
        Value::Array(a) => a.iter().for_each(|x| links(x, false, out)),
        _ => {}
    }
}

#[test]
fn every_example_cell_validates_cleanly() {
    for (name, cell) in cells() {
        let report = validate(&cell, None);
        assert_eq!(report.errors, Vec::<String>::new(), "{name}");
        assert_eq!(report.warnings, Vec::<String>::new(), "{name}");
    }
}

#[test]
fn every_link_points_at_a_cell_in_the_set() {
    let all = cells();
    let ids: BTreeSet<String> = all
        .iter()
        .map(|(_, c)| c["@id"].as_str().unwrap().to_owned())
        .collect();
    for (name, cell) in &all {
        let mut out = BTreeSet::new();
        links(cell, true, &mut out);
        assert!(!out.is_empty(), "{name} links to nothing");
        for id in out {
            assert!(
                ids.contains(&id),
                "{name} links to {id}, which is not in the set"
            );
        }
    }
}

#[test]
fn personal_details_are_samples() {
    let person = &cells()
        .into_iter()
        .find(|(n, _)| n == "person.cell")
        .unwrap()
        .1;
    assert!(person["email"].as_str().unwrap().ends_with("@example.org"));
    assert!(person["telephone"].as_str().unwrap().starts_with("+000"));
}
