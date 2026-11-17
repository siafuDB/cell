//! The `cell` command line: hand-written with the standard library only.
//!
//! [`run`] takes the arguments after the program name and the three standard streams, and
//! returns the exit code, so the whole command can be driven from a test. `main` makes the one
//! call to `std::env::args`.

use std::io::{Read, Write};

use base64::Engine as _;
use base64::alphabet::URL_SAFE;
use base64::engine::DecodePaddingMode;
use base64::engine::general_purpose::{GeneralPurpose, GeneralPurposeConfig, URL_SAFE_NO_PAD};
use serde_json::Value;
use siafudb_cell::{
    SigningKey, VerifyingKey, canonicalize, hash, keys, parse, render, seal, validate, verify_proof,
};

/// The spec version this command implements.
pub const SPEC: &str = "0.1.0 draft, proposed";

/// Exit codes: 0 ok, 1 invalid or failed, 2 usage error.
const OK: u8 = 0;
const FAILED: u8 = 1;
const USAGE: u8 = 2;

/// The three standard streams, so a test can supply its own.
pub struct Io<'a> {
    pub stdin: &'a mut dyn Read,
    pub stdout: &'a mut dyn Write,
    pub stderr: &'a mut dyn Write,
}

impl Io<'_> {
    // A closed stdout or stderr (a pipe into `head`, say) is not worth a panic; the exit
    // code still says how the command went.
    fn out(&mut self, s: &str) {
        let _ = self.stdout.write_all(s.as_bytes());
    }
    fn err(&mut self, s: &str) {
        let _ = self.stderr.write_all(s.as_bytes());
    }
}

struct Flag {
    name: &'static str,
    value: &'static str,
    help: &'static str,
}

struct Command {
    name: &'static str,
    summary: &'static str,
    usage: &'static str,
    /// Names of the positional arguments, in order.
    args: &'static [&'static str],
    flags: &'static [Flag],
    details: &'static str,
    examples: &'static [&'static str],
}

const COMMANDS: &[Command] = &[
    Command {
        name: "check",
        summary: "Validate a cell, and its version chain with --previous",
        usage: "cell check FILE [--previous FILE]",
        args: &["FILE"],
        flags: &[Flag {
            name: "--previous",
            value: "FILE",
            help: "The version before this one: checks the version chain",
        }],
        details: "\
Checks the cell against the rules in SPEC.md: @context, @type, @id, cell:version and
cell:hash are present; every {{field}} in the text names a field; the text is NFC; and
cell:hash matches the content. A warning flags a fact written out in the text instead of
referenced as {{field}} (rule 1, a heuristic).

With --previous, also checks the version chain: the @id is kept, cell:version goes up by
one, and cell:previous is the cell:hash of the version before.

Prints one `warning:` or `error:` line per finding, and nothing for a clean cell.
Exits 1 if there is any error; warnings alone exit 0.",
        examples: &[
            "cell check asthma.cell",
            "cell check v2.cell --previous v1.cell",
        ],
    },
    Command {
        name: "render",
        summary: "Print a cell's text with every {{field}} filled in",
        usage: "cell render FILE [--lang TAG]",
        args: &["FILE"],
        flags: &[Flag {
            name: "--lang",
            value: "TAG",
            help: "The BCP 47 tag to pick from a language map (default: the first text)",
        }],
        details: "\
Fills each {{field}} from the field it names, in one pass: a value that itself looks like
{{...}} is not expanded again. A reference that names no field is left as written; `cell
check` refuses such a cell. When `text` is a language map and TAG is not in it, the first
text is used.",
        examples: &[
            "cell render asthma.cell",
            "cell render asthma.cell --lang sn",
        ],
    },
    Command {
        name: "hash",
        summary: "Print the content hash and say whether it matches cell:hash",
        usage: "cell hash FILE",
        args: &["FILE"],
        flags: &[],
        details: "\
Prints the computed hash on the first line: `sha256-` and the unpadded base64url SHA-256
of the cell in RFC 8785 form, without `cell:hash` and `proof`. The second line says
whether it matches the cell's own `cell:hash`. Exits 1 if the cell has a cell:hash that
does not match; 0 if it matches or the cell has none yet.",
        examples: &["cell hash asthma.cell", "cell hash - < asthma.cell"],
    },
    Command {
        name: "canonical",
        summary: "Print the cell in RFC 8785 (JCS) canonical form",
        usage: "cell canonical FILE",
        args: &["FILE"],
        flags: &[],
        details: "\
Prints the whole cell, hash and proof included, as RFC 8785 JSON: members sorted by their
UTF-16 code units, no whitespace, numbers in the ECMAScript form. The content hash is the
SHA-256 of this form with `cell:hash` and `proof` left out.",
        examples: &["cell canonical asthma.cell"],
    },
    Command {
        name: "keygen",
        summary: "Make an Ed25519 key pair: NAME.key (secret) and NAME.pub",
        usage: "cell keygen --out NAME",
        args: &[],
        flags: &[Flag {
            name: "--out",
            value: "NAME",
            help: "Write NAME.key and NAME.pub (required)",
        }],
        details: "\
Writes two files, each one line of unpadded base64url: NAME.key holds the 32-byte secret
seed and NAME.pub the 32-byte public key. On Unix NAME.key is created with mode 0600.
Refuses to overwrite either file. Never prints the secret key; keep NAME.key private and
share NAME.pub.",
        examples: &["cell keygen --out alice"],
    },
    Command {
        name: "seal",
        summary: "Set cell:hash and sign it; prints the sealed cell",
        usage: "cell seal FILE --key NAME.key",
        args: &["FILE"],
        flags: &[Flag {
            name: "--key",
            value: "NAME.key",
            help: "The secret key from `cell keygen` (required)",
        }],
        details: "\
Drops any old cell:hash and proof, sets cell:hash to the content hash, and signs the
canonical content plus the hash with the key. Prints the sealed cell as JSON on stdout;
the input file is not changed. The proof has the shape of a VC Data Integrity
`eddsa-jcs-2022` proof; it is a prototype, not that algorithm.

Refuses (exit 1, nothing printed) a cell that would still have errors once sealed, such
as a missing @id or a {{field}} that names no field. Warnings go to stderr.",
        examples: &["cell seal draft.cell --key alice.key > asthma.cell"],
    },
    Command {
        name: "verify",
        summary: "Check a cell's proof against a public key",
        usage: "cell verify FILE --pub NAME.pub",
        args: &["FILE"],
        flags: &[Flag {
            name: "--pub",
            value: "NAME.pub",
            help: "The public key from `cell keygen` (required)",
        }],
        details: "\
Checks the proof's Ed25519 signature over the canonical cell without its proof, so the
hash and every field are covered: changing anything after sealing makes it fail. Prints
`proof: valid` and exits 0, or `proof: not valid` and exits 1.",
        examples: &["cell verify asthma.cell --pub alice.pub"],
    },
    Command {
        name: "help",
        summary: "Show this list, or the details of one command",
        usage: "cell help [COMMAND]",
        args: &[],
        flags: &[],
        details: "\
With no COMMAND, lists every command. `cell COMMAND --help` shows the same details as
`cell help COMMAND`.",
        examples: &["cell help", "cell help seal"],
    },
    Command {
        name: "version",
        summary: "Print the crate version and the spec version it implements",
        usage: "cell version",
        args: &[],
        flags: &[],
        details: "Also `cell --version` and `cell -V`.",
        examples: &["cell version"],
    },
];

fn find(name: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| c.name == name)
}

/// The top-level help: every command with its purpose, the exit codes and examples.
pub fn help() -> String {
    let mut s = format!(
        "cell {}: validate, render, hash, seal and verify .cell records\n\
         Implements the .cell spec {SPEC}.\n\n\
         Usage: cell COMMAND [ARGS]\n       cell help COMMAND\n\nCommands:\n",
        env!("CARGO_PKG_VERSION")
    );
    let width = COMMANDS.iter().map(|c| c.name.len()).max().unwrap_or(0);
    for c in COMMANDS {
        s.push_str(&format!("  {:width$}  {}\n", c.name, c.summary));
    }
    s.push_str(
        "\nFILE may be - to read the cell from standard input.\n\n\
         Exit codes:\n  \
         0  ok\n  \
         1  the cell is invalid, or a check, a read or a write failed\n  \
         2  usage error: an unknown command or flag, or a missing argument\n\n\
         Examples:\n  \
         cell check asthma.cell\n  \
         cell check v2.cell --previous v1.cell\n  \
         cell render asthma.cell --lang sn\n  \
         cell keygen --out alice\n  \
         cell seal draft.cell --key alice.key > asthma.cell\n  \
         cell verify asthma.cell --pub alice.pub\n  \
         cat asthma.cell | cell hash -\n\n\
         Run `cell help COMMAND` for the details of one command.\n",
    );
    s
}

fn command_help(c: &Command) -> String {
    let mut s = format!(
        "cell {}: {}\n\nUsage: {}\n\n{}\n\nOptions:\n",
        c.name, c.summary, c.usage, c.details
    );
    let rows: Vec<(String, &str)> = c
        .flags
        .iter()
        .map(|f| (format!("{} {}", f.name, f.value), f.help))
        .chain([("-h, --help".to_owned(), "Show this help")])
        .collect();
    let width = rows.iter().map(|(l, _)| l.len()).max().unwrap_or(0);
    for (l, h) in rows {
        s.push_str(&format!("  {l:width$}  {h}\n"));
    }
    s.push_str("\nExamples:\n");
    for e in c.examples {
        s.push_str(&format!("  {e}\n"));
    }
    s
}

fn version() -> String {
    format!(
        "cell {} (siafudb-cell)\nspec {SPEC}\n",
        env!("CARGO_PKG_VERSION")
    )
}

/// Why a command stopped: a usage error (exit 2) or a failure (exit 1).
enum Stop {
    Usage(String),
    Failed(String),
}

/// The parsed arguments of one command.
struct Parsed {
    args: Vec<String>,
    flags: Vec<(&'static str, String)>,
}

impl Parsed {
    fn flag(&self, name: &str) -> Option<&str> {
        self.flags
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| v.as_str())
    }
    fn required(&self, name: &str, c: &Command) -> Result<&str, Stop> {
        self.flag(name)
            .ok_or_else(|| Stop::Usage(format!("{} needs {name}; usage: {}", c.name, c.usage)))
    }
}

/// Parses `rest` for command `c`. `None` means `--help` was asked for.
fn parse_args(c: &'static Command, rest: &[String]) -> Result<Option<Parsed>, Stop> {
    let mut p = Parsed {
        args: Vec::new(),
        flags: Vec::new(),
    };
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        if a == "-h" || a == "--help" {
            return Ok(None);
        }
        if a.starts_with('-') && a != "-" {
            let (name, inline) = match a.split_once('=') {
                Some((n, v)) => (n, Some(v.to_owned())),
                None => (a.as_str(), None),
            };
            let Some(f) = c.flags.iter().find(|f| f.name == name) else {
                return Err(Stop::Usage(format!("unknown flag {name} for {}", c.name)));
            };
            if p.flag(f.name).is_some() {
                return Err(Stop::Usage(format!("{} given twice", f.name)));
            }
            let value = match inline {
                Some(v) => v,
                None => it.next().cloned().ok_or_else(|| {
                    Stop::Usage(format!("{} needs a value: {} {}", f.name, f.name, f.value))
                })?,
            };
            if value.is_empty() {
                return Err(Stop::Usage(format!("{} needs a value", f.name)));
            }
            p.flags.push((f.name, value));
        } else {
            p.args.push(a.clone());
        }
    }
    if p.args.len() != c.args.len() {
        let what = if p.args.len() < c.args.len() {
            "missing an argument"
        } else {
            "too many arguments"
        };
        return Err(Stop::Usage(format!(
            "{} is {what}; usage: {}",
            c.name, c.usage
        )));
    }
    Ok(Some(p))
}

/// Runs the command line `args` (without the program name) and returns the exit code.
pub fn run(args: &[String], io: &mut Io) -> u8 {
    let Some((cmd, rest)) = args.split_first() else {
        io.err(
            "cell: no command given\nusage: cell COMMAND [ARGS]; run `cell help` for the list\n",
        );
        return USAGE;
    };
    match cmd.as_str() {
        "-h" | "--help" => {
            io.out(&help());
            return OK;
        }
        "-V" | "--version" => {
            io.out(&version());
            return OK;
        }
        _ => {}
    }
    let Some(c) = find(cmd) else {
        io.err(&format!(
            "cell: unknown command `{cmd}`; run `cell help` for the list\n"
        ));
        return USAGE;
    };
    let result = if c.name == "help" {
        run_help(rest, io)
    } else {
        match parse_args(c, rest) {
            Ok(None) => {
                io.out(&command_help(c));
                Ok(OK)
            }
            Ok(Some(p)) => dispatch(c, &p, io),
            Err(e) => Err(e),
        }
    };
    match result {
        Ok(code) => code,
        Err(Stop::Usage(m)) => {
            io.err(&format!(
                "cell: {m}\nrun `cell help` for the list of commands\n"
            ));
            USAGE
        }
        Err(Stop::Failed(m)) => {
            io.err(&format!("error: {m}\n"));
            FAILED
        }
    }
}

fn run_help(rest: &[String], io: &mut Io) -> Result<u8, Stop> {
    match rest {
        [] => io.out(&help()),
        [name] => {
            // `cell help --help` is help on `help`.
            let name = if name == "-h" || name == "--help" {
                "help"
            } else {
                name.as_str()
            };
            match find(name) {
                Some(c) => io.out(&command_help(c)),
                None => return Err(Stop::Usage(format!("no command called `{name}`"))),
            }
        }
        _ => return Err(Stop::Usage("help takes at most one command".into())),
    }
    Ok(OK)
}

fn dispatch(c: &Command, p: &Parsed, io: &mut Io) -> Result<u8, Stop> {
    match c.name {
        "check" => check(p, io),
        "render" => render_cmd(p, io),
        "hash" => hash_cmd(p, io),
        "canonical" => {
            let cell = read_cell(&p.args[0], io)?;
            io.out(&format!("{}\n", canonicalize(&cell)));
            Ok(OK)
        }
        "keygen" => keygen(p.required("--out", c)?, io),
        "seal" => seal_cmd(p, p.required("--key", c)?, io),
        "verify" => verify_cmd(p, p.required("--pub", c)?, io),
        "version" => {
            io.out(&version());
            Ok(OK)
        }
        _ => unreachable!("every command in COMMANDS is dispatched"),
    }
}

fn shown(path: &str) -> &str {
    if path == "-" { "<stdin>" } else { path }
}

fn read_text(path: &str, io: &mut Io) -> Result<String, Stop> {
    let mut text = String::new();
    let read = if path == "-" {
        io.stdin.read_to_string(&mut text).map(|_| ())
    } else {
        std::fs::read_to_string(path).map(|t| text = t)
    };
    read.map_err(|e| Stop::Failed(format!("{}: {e}", shown(path))))?;
    Ok(text)
}

fn read_cell(path: &str, io: &mut Io) -> Result<Value, Stop> {
    let text = read_text(path, io)?;
    let cell = parse(&text).map_err(|e| Stop::Failed(format!("{}: {e}", shown(path))))?;
    if !cell.is_object() {
        return Err(Stop::Failed(format!(
            "{}: a cell is a JSON object",
            shown(path)
        )));
    }
    Ok(cell)
}

fn check(p: &Parsed, io: &mut Io) -> Result<u8, Stop> {
    let file = &p.args[0];
    if p.flag("--previous") == Some("-") && file == "-" {
        return Err(Stop::Usage(
            "only one of FILE and --previous can be - (stdin)".into(),
        ));
    }
    let cell = read_cell(file, io)?;
    let previous = match p.flag("--previous") {
        Some(prev) => Some(read_cell(prev, io)?),
        None => None,
    };
    let report = validate(&cell, previous.as_ref());
    let mut s = String::new();
    for w in &report.warnings {
        s.push_str(&format!("warning: {w}\n"));
    }
    for e in &report.errors {
        s.push_str(&format!("error: {e}\n"));
    }
    io.out(&s);
    Ok(if report.errors.is_empty() { OK } else { FAILED })
}

fn render_cmd(p: &Parsed, io: &mut Io) -> Result<u8, Stop> {
    let path = &p.args[0];
    let cell = read_cell(path, io)?;
    let text = render(&cell, p.flag("--lang").unwrap_or(""))
        .map_err(|e| Stop::Failed(format!("{}: {e}", shown(path))))?;
    io.out(&format!("{text}\n"));
    Ok(OK)
}

fn hash_cmd(p: &Parsed, io: &mut Io) -> Result<u8, Stop> {
    let cell = read_cell(&p.args[0], io)?;
    let h = hash(&cell);
    let (line, code) = match cell.get("cell:hash") {
        None => ("no cell:hash to compare with".to_owned(), OK),
        Some(v) if v.as_str() == Some(h.as_str()) => ("matches cell:hash".to_owned(), OK),
        Some(v) => (
            format!("does not match cell:hash ({})", siafudb_cell::stringify(v)),
            FAILED,
        ),
    };
    io.out(&format!("{h}\n{line}\n"));
    Ok(code)
}

/// Base64url, padded or not.
const B64URL_ANY: GeneralPurpose = GeneralPurpose::new(
    &URL_SAFE,
    GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

fn read_key(path: &str, io: &mut Io) -> Result<[u8; 32], Stop> {
    let text = read_text(path, io)?;
    B64URL_ANY
        .decode(text.trim())
        .ok()
        .and_then(|b| <[u8; 32]>::try_from(b).ok())
        .ok_or_else(|| {
            Stop::Failed(format!(
                "{}: not a 32-byte base64url Ed25519 key",
                shown(path)
            ))
        })
}

fn keygen(name: &str, io: &mut Io) -> Result<u8, Stop> {
    let secret = format!("{name}.key");
    let public = format!("{name}.pub");
    for f in [&secret, &public] {
        if std::path::Path::new(f).exists() {
            return Err(Stop::Failed(format!(
                "{f} already exists; not overwriting it"
            )));
        }
    }
    let (signing, verifying) = keys().map_err(|e| Stop::Failed(e.to_string()))?;
    write_new(&secret, &URL_SAFE_NO_PAD.encode(signing.to_bytes()), true)?;
    write_new(
        &public,
        &URL_SAFE_NO_PAD.encode(verifying.to_bytes()),
        false,
    )?;
    io.out(&format!(
        "wrote {secret} (secret: keep it private) and {public} (public: share it)\n"
    ));
    Ok(OK)
}

/// Creates `path` with `body` and a newline, failing if it exists. A secret file is mode
/// 0600 on Unix from the moment it is created.
fn write_new(path: &str, body: &str, secret: bool) -> Result<(), Stop> {
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        opts.mode(if secret { 0o600 } else { 0o644 });
    }
    #[cfg(not(unix))]
    let _ = secret;
    opts.open(path)
        .and_then(|mut f| f.write_all(format!("{body}\n").as_bytes()))
        .map_err(|e| Stop::Failed(format!("{path}: {e}")))
}

fn seal_cmd(p: &Parsed, key_path: &str, io: &mut Io) -> Result<u8, Stop> {
    let path = &p.args[0];
    if path == "-" && key_path == "-" {
        return Err(Stop::Usage(
            "only one of FILE and --key can be - (stdin)".into(),
        ));
    }
    let cell = read_cell(path, io)?;
    let key = SigningKey::from_bytes(&read_key(key_path, io)?);
    let sealed = seal(&cell, &key);
    let report = validate(&sealed, None);
    let mut notes = String::new();
    for w in &report.warnings {
        notes.push_str(&format!("warning: {w}\n"));
    }
    for e in &report.errors {
        notes.push_str(&format!("error: {e}\n"));
    }
    io.err(&notes);
    if !report.errors.is_empty() {
        return Err(Stop::Failed(format!(
            "{}: not sealed; fix the errors above",
            shown(path)
        )));
    }
    let json = serde_json::to_string_pretty(&sealed)
        .map_err(|e| Stop::Failed(format!("{}: {e}", shown(path))))?;
    io.out(&format!("{json}\n"));
    Ok(OK)
}

fn verify_cmd(p: &Parsed, pub_path: &str, io: &mut Io) -> Result<u8, Stop> {
    let path = &p.args[0];
    if path == "-" && pub_path == "-" {
        return Err(Stop::Usage(
            "only one of FILE and --pub can be - (stdin)".into(),
        ));
    }
    let cell = read_cell(path, io)?;
    let bytes = read_key(pub_path, io)?;
    let key = VerifyingKey::from_bytes(&bytes)
        .map_err(|_| Stop::Failed(format!("{}: not an Ed25519 public key", shown(pub_path))))?;
    if verify_proof(&cell, &key) {
        io.out("proof: valid\n");
        Ok(OK)
    } else {
        io.out("proof: not valid\n");
        Ok(FAILED)
    }
}

/// The command names `cell help` lists, in order.
#[cfg(test)]
fn names() -> Vec<&'static str> {
    COMMANDS.iter().map(|c| c.name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_str(args: &[&str], stdin: &str) -> (u8, String, String) {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let mut input = stdin.as_bytes();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run(
            &args,
            &mut Io {
                stdin: &mut input,
                stdout: &mut out,
                stderr: &mut err,
            },
        );
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    #[test]
    fn every_command_is_listed_and_has_its_own_help() {
        let (code, out, _) = run_str(&["help"], "");
        assert_eq!(code, 0);
        for name in names() {
            assert!(out.contains(&format!("\n  {name} ")), "{name} not listed");
            let (code, detail, _) = run_str(&["help", name], "");
            assert_eq!(code, 0);
            assert!(detail.starts_with(&format!("cell {name}: ")), "{detail}");
        }
    }

    #[test]
    fn flag_parsing() {
        assert_eq!(run_str(&["check"], "").0, 2);
        assert_eq!(run_str(&["check", "a", "b"], "").0, 2);
        assert_eq!(run_str(&["check", "a", "--previous"], "").0, 2);
        assert_eq!(run_str(&["check", "a", "--lang", "en"], "").0, 2);
        assert_eq!(run_str(&["keygen"], "").0, 2);
        assert_eq!(run_str(&["check", "-", "--previous=-"], "{}").0, 2);
        assert_eq!(run_str(&["render", "-", "--lang=en", "--lang=sn"], "").0, 2);
        let (code, out, _) = run_str(
            &["render", "-", "--lang=sn"],
            r#"{"text":{"en":"a","sn":"b"}}"#,
        );
        assert_eq!((code, out.as_str()), (0, "b\n"));
    }
}
