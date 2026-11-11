//! Workspace registry for the history conformance suite: every wire in
//! [`HISTORY_WIRES`](rig_history_conformance::HISTORY_WIRES)
//! must expand `history_conformance_suite!`, or CI fails. Suites compiled
//! into this binary are linked through
//! [`SUITE_WIRES`](super::history_conformance::SUITE_WIRES); the ones that
//! live in a companion crate's own test binary are listed in
//! [`OUT_OF_BINARY_WIRES`] with the file that expands them.

use std::collections::BTreeSet;
use std::path::Path;

use rig_history_conformance::{HISTORY_WIRES, TESTS};

use super::history_conformance;

/// A wire whose suite compiles into a companion crate's test binary.
struct OutOfBinaryWire {
    wire: &'static str,
    suite_file: &'static str,
}

const OUT_OF_BINARY_WIRES: &[OutOfBinaryWire] = &[
    OutOfBinaryWire {
        wire: "candle",
        suite_file: "crates/rig-candle/tests/integration/history_conformance.rs",
    },
    OutOfBinaryWire {
        wire: "gemini_grpc",
        suite_file: "crates/rig-gemini-grpc/tests/integration/history_conformance.rs",
    },
    OutOfBinaryWire {
        wire: "vertexai",
        suite_file: "crates/rig-vertexai/tests/integration/history_conformance.rs",
    },
    OutOfBinaryWire {
        wire: "bedrock_claude",
        suite_file: "crates/rig-bedrock/tests/history_conformance.rs",
    },
    OutOfBinaryWire {
        wire: "bedrock_nova",
        suite_file: "crates/rig-bedrock/tests/history_conformance.rs",
    },
];

#[test]
fn every_completion_wire_has_a_history_suite() {
    let mut covered: BTreeSet<&str> = history_conformance::SUITE_WIRES.iter().copied().collect();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for entry in OUT_OF_BINARY_WIRES {
        let path = root.join(entry.suite_file);
        let source = std::fs::read_to_string(&path).unwrap_or_default();
        if source.contains("history_conformance_suite!")
            && source.contains(&format!("wire: \"{}\"", entry.wire))
        {
            covered.insert(entry.wire);
        }
    }
    let missing: Vec<&&str> = HISTORY_WIRES
        .iter()
        .filter(|wire| !covered.contains(**wire))
        .collect();
    assert!(
        missing.is_empty(),
        "wires without a history_conformance_suite!: {missing:?} (covered: {covered:?})"
    );
    let unknown: Vec<&&str> = covered
        .iter()
        .filter(|wire| !HISTORY_WIRES.contains(*wire))
        .collect();
    assert!(
        unknown.is_empty(),
        "suites name wires missing from HISTORY_WIRES: {unknown:?}"
    );
}

/// Provider modules with no completion wire of their own, so no suite.
const NO_COMPLETION_WIRE: &[&str] = &["internal", "registry", "voyageai"];

/// Every provider module has a suite: some wire in `HISTORY_WIRES` names it
/// as one of its `_`-separated words, so a new provider cannot skip the
/// suite.
#[test]
fn every_provider_module_has_a_history_suite() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/rig-core/src/providers");
    let mut modules: BTreeSet<String> = BTreeSet::new();
    for entry in std::fs::read_dir(&root).expect("the providers directory") {
        let path = entry.expect("a directory entry").path();
        let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        if name != "mod" && (path.is_dir() || path.extension().is_some_and(|ext| ext == "rs")) {
            modules.insert(name.to_owned());
        }
    }
    let missing: Vec<&String> = modules
        .iter()
        .filter(|module| !NO_COMPLETION_WIRE.contains(&module.as_str()))
        .filter(|module| {
            !HISTORY_WIRES
                .iter()
                .any(|wire| wire.split('_').any(|word| word == module.as_str()))
        })
        .collect();
    assert!(
        missing.is_empty(),
        "provider modules without a history suite: {missing:?}"
    );
}

/// Every test `TESTS` names still exists.
#[test]
fn every_named_finding_test_exists() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let missing: Vec<String> = TESTS
        .iter()
        .filter(|(file, test)| {
            !std::fs::read_to_string(root.join(file))
                .unwrap_or_default()
                .contains(&format!("fn {test}("))
        })
        .map(|(file, test)| format!("{file}::{test}"))
        .collect();
    assert!(
        missing.is_empty(),
        "named tests that no longer exist: {missing:?}"
    );
}

/// Every Messages-format dialect runs the suite on its Messages wire, not
/// only on the Chat half its provider module may also serve:
/// `anthropic_<dialect>`, and `anthropic` for Anthropic itself.
#[test]
fn every_messages_dialect_has_a_history_suite() {
    let missing: Vec<String> = rig_core::providers::anthropic::wire::all()
        .map(|dialect| match dialect.name {
            "anthropic" => "anthropic".to_owned(),
            name => format!("anthropic_{name}"),
        })
        .filter(|wire| !history_conformance::SUITE_WIRES.contains(&wire.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "Messages dialects without a history suite: {missing:?}"
    );
}
