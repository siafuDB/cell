//! RFC 8785 (JCS) serialisation, and the plain `JSON.stringify` form that `render` uses.
//!
//! Both write numbers in the ECMAScript form (ECMA-262, Number::toString), which JCS adopts:
//! every number is an IEEE 754 double, written with the shortest digits that read back to
//! the same double.

use serde_json::Value;

/// Writes `v` as JSON with no whitespace. With `sorted`, object members are ordered by the
/// UTF-16 code units of their names (JCS, RFC 8785 section 3.2.3); otherwise they keep
/// their order, as `JSON.stringify` does.
pub(crate) fn write(v: &Value, sorted: bool, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            // Without serde_json's arbitrary_precision every number has an f64 form, and
            // serde_json never holds NaN or an infinity, so this cannot fail.
            out.push_str(&number(n.as_f64().unwrap_or(f64::NAN)));
        }
        Value::String(s) => string(s, out),
        Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write(x, sorted, out);
            }
            out.push(']');
        }
        Value::Object(m) => {
            let mut members: Vec<(&String, &Value)> = m.iter().collect();
            if sorted {
                members.sort_by(|(a, _), (b, _)| a.encode_utf16().cmp(b.encode_utf16()));
            }
            out.push('{');
            for (i, (k, x)) in members.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                string(k, out);
                out.push(':');
                write(x, sorted, out);
            }
            out.push('}');
        }
    }
}

/// A JSON string as `JSON.stringify` and JCS write it: only `"`, `\` and the C0 controls
/// are escaped, using the short forms where JSON has them.
fn string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// The ECMAScript Number::toString form of a finite double. NaN and the infinities have no
/// JSON form; they are written as `null`, as `JSON.stringify` does, though no parsed JSON
/// value can hold them.
pub fn number(x: f64) -> String {
    if !x.is_finite() {
        return "null".to_owned();
    }
    if x == 0.0 {
        return "0".to_owned(); // also -0
    }
    if x < 0.0 {
        return format!("-{}", number(-x));
    }
    // Rust's `{:e}` gives the fewest digits that read back to `x`, as d.ddd…e±x. When several
    // strings of that length read back, ECMAScript wants the one closest to `x`, which Rust
    // does not promise; the correctly rounded string of that length is the closest, so use it
    // whenever it reads back too.
    let shortest = format!("{x:e}");
    let k = shortest
        .split_once('e')
        .map_or(1, |(m, _)| m.len() - m.contains('.') as usize);
    let rounded = format!("{x:.*e}", k - 1);
    let sci = if rounded.parse::<f64>() == Ok(x) {
        rounded
    } else {
        shortest
    };
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i32;
    let n = exp.parse::<i32>().unwrap_or(0) + 1; // x = 0.d1d2…dk × 10^n
    if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        let (a, b) = digits.split_at(n as usize);
        format!("{a}.{b}")
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let e = n - 1;
        let sign = if e < 0 { '-' } else { '+' };
        let (first, rest) = digits.split_at(1);
        if rest.is_empty() {
            format!("{first}e{sign}{}", e.abs())
        } else {
            format!("{first}.{rest}e{sign}{}", e.abs())
        }
    }
}
