//! The README's quickstart, kept compiling: `cargo run --example quickstart`.

use serde_json::json;
use siafudb_cell::{keys, render, seal, validate, verify_proof};

fn main() {
    // One record, facts and words together.
    let draft = json!({
        "@context": "https://schema.org",
        "@type": "MedicalCondition",
        "@id": "urn:cell:01JD3Q7Z8K2V5M",
        "name": "Asthma",
        "possibleTreatment": { "@type": "Drug", "name": "Salbutamol inhaler" },
        "dateCreated": "2024-03-11",
        "text": "Diagnosed on {{dateCreated}}. Carry the {{possibleTreatment.name}} when walking far.",
        "cell:version": 1
    });

    // Hash it and sign it with the owner's key.
    let (secret, public) = keys().expect("system randomness");
    let mut cell = seal(&draft, &secret);
    println!("hash:     {}", cell["cell:hash"]);
    println!("valid:    {}", validate(&cell, None).errors.is_empty());
    println!("signed:   {}", verify_proof(&cell, &public));
    println!("reads:    {}", render(&cell, "en").unwrap());

    // Change one fact, and every sentence that uses it changes with it...
    cell["possibleTreatment"]["name"] = json!("Budesonide inhaler");
    println!("reads:    {}", render(&cell, "en").unwrap());

    // ...but the hash no longer matches, so the edit is caught until it is resealed.
    let report = validate(&cell, None);
    println!("tampered: {:?}", report.errors);
}
