//! Stress tests for the Bundu Cell 0.1.0 draft. Each test records what the draft does under a
//! hostile case, so a finding is a test result, not an opinion.
//!
//! ```sh
//! cargo test
//! CELL_PG="psql -h HOST -p PORT -U postgres" cargo test -- --ignored   # the database round trip
//! ```

use std::io::Write as _;
use std::process::{Command, Stdio};
use std::sync::LazyLock;

use serde_json::{Map, Value, json};
use siafudb_cell::{
    Report, SigningKey, VerifyingKey, canonicalize, hash, keys, parse, render, seal, validate,
    verify_proof,
};

fn base() -> Value {
    json!({
        "@context": ["https://schema.org", "https://standards.bundu.org/cell/v0.1"],
        "@type": "MedicalCondition",
        "@id": "urn:cell:01JD3Q7Z8K2V5M",
        "name": "Asthma",
        "possibleTreatment": { "@type": "Drug", "name": "Salbutamol inhaler" },
        "dateCreated": "2024-03-11",
        "text": "Diagnosed on {{dateCreated}}. Carry the {{possibleTreatment.name}} when walking far.",
        "cell:version": 1,
    })
}

/// `{ ...cell, ...patch }`: a member already there keeps its place, a new one goes last.
fn with(cell: &Value, patch: Value) -> Value {
    let mut out = cell.as_object().cloned().unwrap_or_default();
    for (k, v) in patch.as_object().cloned().unwrap_or_default() {
        out.insert(k, v);
    }
    Value::Object(out)
}

static KEYS: LazyLock<(SigningKey, VerifyingKey)> = LazyLock::new(|| keys().expect("a key pair"));

fn sealed(c: &Value) -> Value {
    seal(c, &KEYS.0)
}

fn ok() -> Report {
    Report::default()
}

#[test]
fn a_sealed_cell_validates_verifies_and_renders() {
    let c = sealed(&base());
    assert_eq!(validate(&c, None), ok());
    assert!(verify_proof(&c, &KEYS.1));
    assert_eq!(
        render(&c, "").unwrap(),
        "Diagnosed on 2024-03-11. Carry the Salbutamol inhaler when walking far."
    );
}

#[test]
fn changing_a_fact_changes_the_rendered_prose_and_breaks_the_old_hash() {
    let c = sealed(&base());
    let edited = with(
        &c,
        json!({ "possibleTreatment": { "@type": "Drug", "name": "Budesonide" } }),
    );
    assert!(
        validate(&edited, None)
            .errors
            .join(",")
            .contains("cell:hash does not match")
    );
    assert!(
        render(&sealed(&edited), "")
            .unwrap()
            .contains("Carry the Budesonide")
    );
}

#[test]
fn a_reference_to_a_missing_field_is_refused() {
    let c = sealed(&with(
        &base(),
        json!({ "text": "Allergic to {{allergen}}." }),
    ));
    assert_eq!(
        validate(&c, None).errors,
        ["text: {{allergen}} names no field"]
    );
}

#[test]
fn rule_1_facts_once_is_a_heuristic_it_catches_exact_copies_and_misses_paraphrase() {
    let exact = sealed(&with(
        &base(),
        json!({ "text": "Diagnosed on 2024-03-11." }),
    ));
    assert!(
        validate(&exact, None)
            .warnings
            .join(",")
            .contains("restates dateCreated")
    );
    let paraphrase = sealed(&with(
        &base(),
        json!({ "text": "Diagnosed in March 2024." }),
    ));
    assert!(
        validate(&paraphrase, None).warnings.is_empty(),
        "the paraphrase slips through"
    );
}

#[test]
fn a_field_value_that_looks_like_a_reference_is_not_expanded_twice() {
    let c = sealed(&with(
        &base(),
        json!({ "name": "{{proof}}", "text": "Condition: {{name}}." }),
    ));
    assert_eq!(render(&c, "").unwrap(), "Condition: {{proof}}.");
}

#[test]
fn key_order_and_whitespace_do_not_change_the_hash_jcs() {
    let c = sealed(&base());
    let reversed: Map<String, Value> = c
        .as_object()
        .unwrap()
        .iter()
        .rev()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let mut text = Vec::new();
    let fmt = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser = serde_json::Serializer::with_formatter(&mut text, fmt);
    serde::Serialize::serialize(&Value::Object(reversed), &mut ser).unwrap();
    let reordered = parse(std::str::from_utf8(&text).unwrap()).unwrap();
    assert_eq!(Value::String(hash(&reordered)), c["cell:hash"]);
}

#[test]
fn duplicate_member_names_are_refused_i_json_rfc_7493() {
    let e = parse(r#"{"name":"Asthma","name":"Healthy"}"#).unwrap_err();
    assert!(e.to_string().contains(r#"duplicate member "name""#), "{e}");
}

#[test]
fn unnormalised_unicode_changes_the_hash_nfc_is_required() {
    let nfc = with(&base(), json!({ "name": "Ch\u{e9}" }));
    let nfd = with(&base(), json!({ "name": "Che\u{301}" }));
    assert_ne!(hash(&nfc), hash(&nfd), "same name on screen, two hashes");
    let c = sealed(&with(
        &base(),
        json!({ "text": "Re\u{301}sume\u{301} of {{name}}" }),
    ));
    assert!(validate(&c, None).errors.join(",").contains("not NFC"));
}

#[test]
fn integers_above_2_53_are_rounded_by_jcs_so_the_hash_differs_by_language() {
    let text = r#"{"@id":"urn:cell:x","nationalId":12345678901234567}"#;
    let jcs = canonicalize(&parse(text).unwrap());
    assert_eq!(
        jcs, r#"{"@id":"urn:cell:x","nationalId":12345678901234568}"#,
        "JCS reads every number as a double, so the integer is rounded (as JavaScript does)"
    );
    let out = Command::new("python3")
        .args([
            "-c",
            "import json,sys;print(json.dumps(json.loads(sys.argv[1]),separators=(',',':'),sort_keys=True))",
            text,
        ])
        .output()
        .expect("python3 runs");
    let py = String::from_utf8(out.stdout).unwrap();
    assert_ne!(
        py.trim(),
        jcs,
        "Python keeps the exact integer; the two canonical forms disagree"
    );
}

#[test]
fn offline_edits_on_two_devices_fork_the_chain_both_versions_are_valid() {
    let v1 = sealed(&base());
    let next = |patch: Value| {
        let c = with(&v1, patch);
        sealed(&with(
            &c,
            json!({ "cell:version": 2, "cell:previous": v1["cell:hash"] }),
        ))
    };
    let phone = next(json!({ "name": "Asthma (mild)" }));
    let tablet = next(json!({ "dateCreated": "2024-03-12" }));
    assert!(validate(&phone, Some(&v1)).errors.is_empty());
    assert!(validate(&tablet, Some(&v1)).errors.is_empty());
    assert_ne!(
        phone["cell:hash"], tablet["cell:hash"],
        "two different version 2s: the draft has no merge rule"
    );
}

#[test]
fn deleting_an_old_versions_content_leaves_the_chain_checkable_from_hashes_alone() {
    let v1 = sealed(&base());
    let v2 = sealed(&with(
        &v1,
        json!({ "name": "Asthma (mild)", "cell:version": 2, "cell:previous": v1["cell:hash"] }),
    ));
    let tombstone = json!({ "@id": v1["@id"], "cell:version": 1, "cell:hash": v1["cell:hash"] });
    assert_eq!(validate(&v2, Some(&tombstone)).errors, Vec::<String>::new());
}

#[test]
fn text_in_several_languages_a_json_ld_language_map_references_checked_in_each() {
    let c = sealed(&with(
        &base(),
        json!({ "text": {
            "en": "Carry the {{possibleTreatment.name}}.",
            "sn": "Takura {{possibleTreatment.name}} yako.",
        } }),
    ));
    assert_eq!(validate(&c, None).errors, Vec::<String>::new());
    assert_eq!(render(&c, "sn").unwrap(), "Takura Salbutamol inhaler yako.");
    let broken = sealed(&with(
        &base(),
        json!({ "text": {
            "en": "Carry the {{possibleTreatment.name}}.",
            "sn": "Takura {{mushonga}}.",
        } }),
    ));
    assert_eq!(
        validate(&broken, None).errors,
        ["text (sn): {{mushonga}} names no field"]
    );
}

#[test]
#[ignore = "needs Postgres: set CELL_PG to a psql command and run with --ignored"]
fn database_round_trip_jsonb_keeps_the_meaning_and_the_hash_not_the_bytes() {
    let pg = std::env::var("CELL_PG").expect("set CELL_PG to a psql command");
    let cell = sealed(&with(
        &base(),
        json!({ "dose": 1.1, "@context": base()["@context"] }),
    ));
    let file = serde_json::to_string_pretty(&cell).unwrap();
    let psql = |sql: &str| {
        let mut child = Command::new("sh")
            .args(["-c", &format!("{pg} -tA -v ON_ERROR_STOP=1")])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("psql runs");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(sql.as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success(), "psql failed");
        String::from_utf8(out.stdout).unwrap().trim().to_owned()
    };
    let lit = file.replace('\'', "''");
    let as_jsonb = psql(&format!("select '{lit}'::jsonb::text;"));
    let as_json = psql(&format!("select '{lit}'::json::text;"));
    assert_eq!(as_json, file, "a json column keeps the exact bytes");
    assert_ne!(
        as_jsonb, file,
        "a jsonb column rewrites them (key order, spacing)"
    );
    assert_eq!(
        Value::String(hash(&parse(&as_jsonb).unwrap())),
        parse(&file).unwrap()["cell:hash"],
        "the hash survives jsonb"
    );
    assert_eq!(
        validate(&parse(&as_jsonb).unwrap(), None).errors,
        Vec::<String>::new()
    );
}
