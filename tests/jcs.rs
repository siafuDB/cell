//! RFC 8785 (JCS) test vectors: section 3.2.2 (the sample), section 3.2.3 (sorting) and
//! appendix B (number serialisation).

use serde_json::{Value, json};
use siafudb_cell::{canonicalize, number, parse, stringify};

#[test]
fn rfc8785_section_3_2_2_sample() {
    let input = r#"{
      "numbers": [333333333.33333329, 1E30, 4.50, 2e-3, 0.000000000000000000000000001],
      "string": "\u20ac$\u000F\u000aA'\u0042\u0022\u005c\\\"\/",
      "literals": [null, true, false]
    }"#;
    assert_eq!(
        canonicalize(&parse(input).unwrap()),
        r#"{"literals":[null,true,false],"numbers":[333333333.3333333,1e+30,4.5,0.002,1e-27],"string":"€$\u000f\nA'B\"\\\\\"/"}"#
    );
}

#[test]
fn rfc8785_section_3_2_3_sorts_by_utf16_code_units() {
    let input = r#"{
      "\u20ac": "Euro Sign",
      "\r": "Carriage Return",
      "\ufb33": "Hebrew Letter Dalet With Dagesh",
      "1": "One",
      "\ud83d\ude00": "Emoji: Grinning Face",
      "\u0080": "Control",
      "\u00f6": "Latin Small Letter O With Diaeresis"
    }"#;
    let values: Vec<Value> = {
        let c = canonicalize(&parse(input).unwrap());
        // Read back with the order kept, to list the values in canonical order.
        let v: Value = serde_json::from_str(&c).unwrap();
        v.as_object().unwrap().values().cloned().collect()
    };
    assert_eq!(
        values,
        [
            "Carriage Return",
            "One",
            "Control",
            "Latin Small Letter O With Diaeresis",
            "Euro Sign",
            "Emoji: Grinning Face",
            "Hebrew Letter Dalet With Dagesh",
        ]
    );
}

#[test]
fn rfc8785_appendix_b_numbers() {
    let cases: [(u64, &str); 23] = [
        (0x0000000000000000, "0"),
        (0x8000000000000000, "0"),
        (0x0000000000000001, "5e-324"),
        (0x8000000000000001, "-5e-324"),
        (0x7fefffffffffffff, "1.7976931348623157e+308"),
        (0xffefffffffffffff, "-1.7976931348623157e+308"),
        (0x4340000000000000, "9007199254740992"),
        (0xc340000000000000, "-9007199254740992"),
        (0x4430000000000000, "295147905179352830000"),
        (0x44b52d02c7e14af5, "9.999999999999997e+22"),
        (0x44b52d02c7e14af6, "1e+23"),
        (0x44b52d02c7e14af7, "1.0000000000000001e+23"),
        (0x444b1ae4d6e2ef4e, "999999999999999700000"),
        (0x444b1ae4d6e2ef4f, "999999999999999900000"),
        (0x444b1ae4d6e2ef50, "1e+21"),
        (0x3eb0c6f7a0b5ed8c, "9.999999999999997e-7"),
        (0x3eb0c6f7a0b5ed8d, "0.000001"),
        (0x41b3de4355555553, "333333333.3333332"),
        (0x41b3de4355555554, "333333333.33333325"),
        (0x41b3de4355555555, "333333333.3333333"),
        (0x41b3de4355555556, "333333333.3333334"),
        (0x41b3de4355555557, "333333333.33333343"),
        (0xbecbf647612f3696, "-0.0000033333333333333333"),
    ];
    for (bits, want) in cases {
        assert_eq!(number(f64::from_bits(bits)), want, "{bits:#018x}");
    }
    assert_eq!(
        number(f64::from_bits(0x43143ff3c1cb0959)),
        "1424953923781206.2"
    );
}

#[test]
fn rfc8785_appendix_b_nan_and_infinity_have_no_json_form() {
    // A parsed cell cannot hold them: serde_json refuses them, and so does the parser.
    assert!(serde_json::Number::from_f64(f64::NAN).is_none());
    assert!(serde_json::Number::from_f64(f64::INFINITY).is_none());
    assert!(parse("[1e400]").is_err());
}

#[test]
fn integers_and_doubles_print_alike() {
    assert_eq!(
        canonicalize(&json!([1, 1.0, -0.0, 100, 1e21, 1e-7])),
        "[1,1,0,100,1e+21,1e-7]"
    );
}

#[test]
fn stringify_keeps_member_order_and_compacts() {
    let v = parse(r#"{ "b": 1, "a": [ 2.50, "x" ] }"#).unwrap();
    assert_eq!(stringify(&v), r#"{"b":1,"a":[2.5,"x"]}"#);
    assert_eq!(canonicalize(&v), r#"{"a":[2.5,"x"],"b":1}"#);
}
