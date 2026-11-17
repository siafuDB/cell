//! The `cell` command, run as a separate process: every command's happy path, help, exit
//! codes, standard input, and the keygen, seal and verify round trip.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::{Value, json};
use siafudb_cell::{hash, keys, parse, seal};

/// A fresh, empty directory for one test, removed when it is dropped.
struct Dir(PathBuf);

impl Dir {
    fn new(test: &str) -> Dir {
        let d = std::env::temp_dir().join(format!("cell-cli-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        Dir(d)
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
    fn write(&self, name: &str, cell: &Value) -> String {
        let p = self.path(name);
        std::fs::write(&p, serde_json::to_string_pretty(cell).unwrap()).unwrap();
        s(&p)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn s(p: &Path) -> String {
    p.to_str().unwrap().to_owned()
}

fn cell(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cell"))
        .args(args)
        .output()
        .unwrap()
}

fn cell_stdin(args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cell"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // A usage error exits before reading stdin, so the write can meet a closed pipe; the
    // exit status is what the test checks.
    if let Err(e) = child.stdin.take().unwrap().write_all(input.as_bytes()) {
        assert_eq!(e.kind(), std::io::ErrorKind::BrokenPipe, "{e}");
    }
    child.wait_with_output().unwrap()
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn draft() -> Value {
    json!({
        "@context": "https://schema.org",
        "@type": "MedicalCondition",
        "@id": "urn:cell:cli",
        "name": "Asthma",
        "dateCreated": "2024-03-11",
        "text": { "en": "Diagnosed on {{dateCreated}}.", "sn": "{{name}} kubva {{dateCreated}}." },
        "cell:version": 1,
    })
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
    let dir = Dir::new("check-render");
    let p = dir.write("ok.cell", &c);

    let o = cell(&["check", &p]);
    assert!(o.status.success());
    assert_eq!(out(&o), "");

    let o = cell(&["render", &p]);
    assert!(o.status.success());
    assert_eq!(out(&o), "About Asthma.\n");
}

#[test]
fn check_fails_on_errors_and_bad_usage() {
    let dir = Dir::new("check-bad");
    let p = dir.write("bad.cell", &json!({ "text": "{{nothing}}" }));
    let o = cell(&["check", &p]);
    assert_eq!(o.status.code(), Some(1));
    let stdout = out(&o);
    assert!(stdout.contains("error: missing @id"), "{stdout}");
    assert!(
        stdout.contains("error: text: {{nothing}} names no field"),
        "{stdout}"
    );

    assert_eq!(cell(&[]).status.code(), Some(2));
    assert_eq!(cell(&["seal", "x.cell"]).status.code(), Some(2));
}

#[test]
fn help_lists_every_command_with_exit_codes_and_examples() {
    for flag in ["help", "-h", "--help"] {
        let o = cell(&[flag]);
        assert!(o.status.success(), "{flag}");
        let text = out(&o);
        for name in [
            "check",
            "render",
            "hash",
            "canonical",
            "keygen",
            "seal",
            "verify",
            "help",
            "version",
        ] {
            assert!(text.contains(&format!("\n  {name} ")), "{flag}: {name}");
        }
        assert!(text.contains("Exit codes:"), "{text}");
        assert!(text.contains("Examples:"), "{text}");
        assert!(text.contains("2  usage error"), "{text}");
    }
}

#[test]
fn help_for_one_command_two_ways() {
    let a = cell(&["help", "seal"]);
    let b = cell(&["seal", "--help"]);
    let c = cell(&["seal", "-h"]);
    assert!(a.status.success() && b.status.success() && c.status.success());
    assert_eq!(out(&a), out(&b));
    assert_eq!(out(&a), out(&c));
    let text = out(&a);
    assert!(
        text.starts_with("cell seal: ")
            && text.contains("Usage: cell seal FILE --key NAME.key")
            && text.contains("--key NAME.key"),
        "{text}"
    );
    assert_eq!(cell(&["help", "nope"]).status.code(), Some(2));
}

#[test]
fn version_names_the_crate_and_the_spec() {
    let want = format!(
        "cell {} (siafudb-cell)\nspec 0.1.0 draft, proposed\n",
        env!("CARGO_PKG_VERSION")
    );
    for flag in ["--version", "version", "-V"] {
        let o = cell(&[flag]);
        assert!(o.status.success(), "{flag}");
        assert_eq!(out(&o), want, "{flag}");
    }
}

#[test]
fn unknown_command_or_flag_is_a_usage_error() {
    let o = cell(&["frobnicate"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(
        err(&o).contains("unknown command `frobnicate`"),
        "{}",
        err(&o)
    );
    assert!(err(&o).contains("cell help"), "{}", err(&o));
    assert_eq!(out(&o), "");

    let o = cell(&["check", "x.cell", "--frob"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(err(&o).contains("unknown flag --frob"), "{}", err(&o));
    assert!(err(&o).contains("cell help"), "{}", err(&o));

    assert_eq!(cell(&["check"]).status.code(), Some(2));
    assert_eq!(cell(&["render", "a", "b"]).status.code(), Some(2));
    assert_eq!(cell(&["render", "a", "--lang"]).status.code(), Some(2));
}

#[test]
fn a_missing_file_fails_with_exit_1_on_stderr() {
    let o = cell(&["check", "/no/such/dir/x.cell"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(
        err(&o).starts_with("error: /no/such/dir/x.cell"),
        "{}",
        err(&o)
    );
    assert_eq!(out(&o), "");
}

#[test]
fn render_picks_a_language() {
    let dir = Dir::new("render-lang");
    let p = dir.write("d.cell", &draft());
    assert_eq!(out(&cell(&["render", &p])), "Diagnosed on 2024-03-11.\n");
    assert_eq!(
        out(&cell(&["render", &p, "--lang", "sn"])),
        "Asthma kubva 2024-03-11.\n"
    );
    assert_eq!(
        out(&cell(&["render", &p, "--lang=sn"])),
        "Asthma kubva 2024-03-11.\n"
    );
}

#[test]
fn hash_and_canonical() {
    let dir = Dir::new("hash");
    let d = draft();
    let p = dir.write("d.cell", &d);
    let o = cell(&["hash", &p]);
    assert!(o.status.success());
    assert_eq!(
        out(&o),
        format!("{}\nno cell:hash to compare with\n", hash(&d))
    );

    let (key, _) = keys().unwrap();
    let sealed = seal(&d, &key);
    let p = dir.write("s.cell", &sealed);
    let o = cell(&["hash", &p]);
    assert!(o.status.success());
    assert_eq!(out(&o), format!("{}\nmatches cell:hash\n", hash(&d)));

    let mut stale = sealed.clone();
    stale["name"] = json!("Asthma (mild)");
    let p = dir.write("stale.cell", &stale);
    let o = cell(&["hash", &p]);
    assert_eq!(o.status.code(), Some(1));
    assert!(out(&o).contains("does not match cell:hash"), "{}", out(&o));

    let p = dir.write("c.cell", &json!({ "b": [1.0, "x"], "a": 2 }));
    let o = cell(&["canonical", &p]);
    assert!(o.status.success());
    assert_eq!(out(&o), "{\"a\":2,\"b\":[1,\"x\"]}\n");
}

#[test]
fn stdin_as_dash() {
    let text = serde_json::to_string(&draft()).unwrap();
    let o = cell_stdin(&["hash", "-"], &text);
    assert!(o.status.success());
    assert!(out(&o).starts_with(&hash(&draft())), "{}", out(&o));

    let o = cell_stdin(&["render", "-", "--lang", "en"], &text);
    assert_eq!(out(&o), "Diagnosed on 2024-03-11.\n");

    let o = cell_stdin(&["check", "-"], &text);
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(out(&o), "error: missing cell:hash\n");

    let o = cell_stdin(&["canonical", "-"], "{\"name\":\"x\",\"name\":\"y\"}");
    assert_eq!(o.status.code(), Some(1));
    assert!(
        err(&o).contains("<stdin>: I-JSON: duplicate member"),
        "{}",
        err(&o)
    );

    assert_eq!(
        cell_stdin(&["check", "-", "--previous", "-"], &text)
            .status
            .code(),
        Some(2)
    );
}

#[test]
fn keygen_seal_verify_round_trip_and_tampering() {
    let dir = Dir::new("round-trip");
    let name = s(&dir.path("alice"));
    let o = cell(&["keygen", "--out", &name]);
    assert!(o.status.success(), "{}", err(&o));
    let secret = std::fs::read_to_string(format!("{name}.key")).unwrap();
    let public = std::fs::read_to_string(format!("{name}.pub")).unwrap();
    assert_eq!(secret.trim().len(), 43, "32 bytes, unpadded base64url");
    assert_eq!(public.trim().len(), 43);
    assert!(
        !out(&o).contains(secret.trim()) && !err(&o).contains(secret.trim()),
        "the secret key is never printed"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(format!("{name}.key"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    let draft_path = dir.write("draft.cell", &draft());
    let o = cell(&["seal", &draft_path, "--key", &format!("{name}.key")]);
    assert!(o.status.success(), "{}", err(&o));
    let sealed = parse(&out(&o)).unwrap();
    assert_eq!(sealed["cell:hash"], json!(hash(&draft())));
    assert_eq!(sealed["proof"]["type"], "DataIntegrityProof");
    let sealed_path = dir.write("sealed.cell", &sealed);

    assert_eq!(out(&cell(&["check", &sealed_path])), "");
    let pubkey = format!("{name}.pub");
    let o = cell(&["verify", &sealed_path, "--pub", &pubkey]);
    assert!(o.status.success());
    assert_eq!(out(&o), "proof: valid\n");

    // Change a fact after sealing: the proof no longer holds.
    let mut tampered = sealed.clone();
    tampered["dateCreated"] = json!("2024-03-12");
    let p = dir.write("tampered.cell", &tampered);
    let o = cell(&["verify", &p, "--pub", &pubkey]);
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(out(&o), "proof: not valid\n");

    // Re-hashing without re-signing does not help either.
    let mut rehashed = tampered.clone();
    rehashed["cell:hash"] = json!(hash(&tampered));
    let p = dir.write("rehashed.cell", &rehashed);
    assert_eq!(
        cell(&["verify", &p, "--pub", &pubkey]).status.code(),
        Some(1)
    );

    // Another key's proof does not verify.
    let other = s(&dir.path("bob"));
    assert!(cell(&["keygen", "--out", &other]).status.success());
    let o = cell(&["verify", &sealed_path, "--pub", &format!("{other}.pub")]);
    assert_eq!(o.status.code(), Some(1));

    // Sealing from stdin, then a new version checked against the old one.
    let mut v2 = sealed.clone();
    v2["name"] = json!("Asthma (mild)");
    v2["cell:version"] = json!(2);
    v2["cell:previous"] = sealed["cell:hash"].clone();
    let o = cell_stdin(
        &["seal", "-", "--key", &format!("{name}.key")],
        &serde_json::to_string(&v2).unwrap(),
    );
    assert!(o.status.success(), "{}", err(&o));
    let v2_path = dir.write("v2.cell", &parse(&out(&o)).unwrap());
    let o = cell(&["check", &v2_path, "--previous", &sealed_path]);
    assert!(o.status.success(), "{}", out(&o));
    let o = cell(&["check", &sealed_path, "--previous", &v2_path]);
    assert_eq!(o.status.code(), Some(1));
    assert!(
        out(&o).contains("cell:version must go up by one"),
        "{}",
        out(&o)
    );
}

#[test]
fn keygen_refuses_to_overwrite() {
    let dir = Dir::new("keygen-twice");
    let name = s(&dir.path("k"));
    assert!(cell(&["keygen", "--out", &name]).status.success());
    let before = std::fs::read(format!("{name}.key")).unwrap();
    let o = cell(&["keygen", "--out", &name]);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("already exists"), "{}", err(&o));
    assert_eq!(std::fs::read(format!("{name}.key")).unwrap(), before);

    // A stray NAME.pub alone also stops it, before any secret is written.
    let lone = s(&dir.path("lone"));
    std::fs::write(format!("{lone}.pub"), "x").unwrap();
    assert_eq!(cell(&["keygen", "--out", &lone]).status.code(), Some(1));
    assert!(!Path::new(&format!("{lone}.key")).exists());

    assert_eq!(cell(&["keygen"]).status.code(), Some(2));
}

#[test]
fn seal_refuses_a_cell_with_errors_and_a_bad_key() {
    let dir = Dir::new("seal-bad");
    let name = s(&dir.path("k"));
    assert!(cell(&["keygen", "--out", &name]).status.success());
    let p = dir.write("bad.cell", &json!({ "@id": "x", "text": "{{nope}}" }));
    let o = cell(&["seal", &p, "--key", &format!("{name}.key")]);
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(out(&o), "");
    assert!(err(&o).contains("names no field"), "{}", err(&o));

    let good = dir.write("good.cell", &draft());
    let junk = dir.path("junk.key");
    std::fs::write(&junk, "not a key").unwrap();
    let o = cell(&["seal", &good, "--key", &s(&junk)]);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("not a 32-byte base64url"), "{}", err(&o));
}
