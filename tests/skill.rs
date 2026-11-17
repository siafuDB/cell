//! Keeps the skill served from this repository in step with the code: the plugin manifests
//! parse, carry the crate version, every command `cell help` lists is in the skill, the
//! in-repo copy matches the plugin skill, the skill's example cell is valid, and the README
//! shows the current `cell help`.

use std::path::Path;
use std::process::Command;

use serde_json::Value;
use siafudb_cell::{parse, validate};

const SKILL: &str = "plugin/skills/cell/SKILL.md";

fn read(path: &str) -> String {
    let full = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn json(path: &str) -> Value {
    serde_json::from_str(&read(path)).unwrap_or_else(|e| panic!("{path} is not JSON: {e}"))
}

/// The command names under "Commands:" in `cell help`.
fn listed_commands() -> Vec<String> {
    let out = Command::new(env!("CARGO_BIN_EXE_cell"))
        .arg("help")
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    let names: Vec<String> = text
        .lines()
        .skip_while(|l| *l != "Commands:")
        .skip(1)
        .take_while(|l| !l.trim().is_empty())
        .filter_map(|l| l.split_whitespace().next().map(str::to_owned))
        .collect();
    assert!(names.len() >= 9, "too few commands parsed: {names:?}");
    names
}

#[test]
fn manifests_parse_as_json() {
    let market = json(".claude-plugin/marketplace.json");
    assert_eq!(market["name"], "siafudb-cell");
    let plugins = market["plugins"].as_array().unwrap();
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0]["name"], "cell");
    assert_eq!(plugins[0]["source"], "./plugin");

    let plugin = json("plugin/plugin.json");
    assert_eq!(plugin["name"], "cell");
    assert_eq!(plugin["license"], "Apache-2.0");
    assert_eq!(plugin["repository"], "https://github.com/siafuDB/cell");
    assert!(plugin.get("hooks").is_none(), "Markdown only: no hooks");
}

#[test]
fn plugin_version_is_the_crate_version() {
    assert_eq!(
        json("plugin/plugin.json")["version"],
        env!("CARGO_PKG_VERSION")
    );
}

#[test]
fn both_plugin_manifests_are_identical() {
    // Claude Code reads plugin/.claude-plugin/plugin.json; plugin/plugin.json mirrors the
    // mzizi-registry layout. They must not drift.
    assert_eq!(
        read("plugin/.claude-plugin/plugin.json"),
        read("plugin/plugin.json")
    );
}

#[test]
fn every_listed_command_is_in_the_skill() {
    let skill = read(SKILL);
    for name in listed_commands() {
        assert!(
            skill.contains(&format!("`cell {name}")),
            "{SKILL} does not show `cell {name}`"
        );
    }
}

#[test]
fn the_skill_has_frontmatter() {
    let skill = read(SKILL);
    let front = skill
        .strip_prefix("---\n")
        .and_then(|s| s.split_once("\n---\n"))
        .map(|(f, _)| f)
        .expect("frontmatter between --- lines");
    assert!(front.lines().any(|l| l == "name: cell"), "{front}");
    assert!(
        front.lines().any(|l| l.starts_with("description: Use ")),
        "{front}"
    );
}

#[test]
fn the_skill_example_cell_is_valid() {
    let skill = read(SKILL);
    let (_, after) = skill
        .split_once("## A minimal valid cell")
        .expect("a minimal valid cell section");
    let (_, block) = after.split_once("```json\n").expect("a json block");
    let (text, _) = block.split_once("```").unwrap();
    let cell = parse(text).unwrap();
    let report = validate(&cell, None);
    assert_eq!(report.errors, Vec::<String>::new());
    assert_eq!(report.warnings, Vec::<String>::new());
}

#[test]
fn the_readme_shows_the_current_help() {
    let out = Command::new(env!("CARGO_BIN_EXE_cell"))
        .arg("help")
        .output()
        .unwrap();
    let help = String::from_utf8(out.stdout).unwrap();
    assert!(
        read("README.md").contains(&format!("```text\n{help}```")),
        "paste the output of `cell help` into the README's CLI section"
    );
}
