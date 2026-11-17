//! The `cell` command: `check` and `render` on a file.

use std::process::Command;

use serde_json::json;
use siafudb_cell::{keys, seal};

fn write_cell(name: &str, cell: &serde_json::Value) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("cell-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, serde_json::to_string_pretty(cell).unwrap()).unwrap();
    path
}

fn cell(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_cell"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn check_and_render_a_sealed_cell() {
    let (key, _) = keys().unwrap();
    let c = seal(
        &json!({
            "@context": "https://schema.org",
            "@type": "Thing",
            "@id": "urn:cell:cli",
            "name": "Asthma",
            "text": "About {{name}}.",
            "cell:version": 1,
        }),
        &key,
    );
    let path = write_cell("ok.cell", &c);
    let p = path.to_str().unwrap();

    let out = cell(&["check", p]);
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout), "");

    let out = cell(&["render", p]);
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout), "About Asthma.\n");
}

#[test]
fn check_fails_on_errors_and_bad_usage() {
    let path = write_cell("bad.cell", &json!({ "text": "{{nothing}}" }));
    let out = cell(&["check", path.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("error: missing @id"), "{stdout}");
    assert!(
        stdout.contains("error: text: {{nothing}} names no field"),
        "{stdout}"
    );

    assert_eq!(cell(&[]).status.code(), Some(2));
    assert_eq!(cell(&["seal", "x.cell"]).status.code(), Some(2));
}
