//! Workspace registry for the wire-conformance corpus: every streaming wire
//! family in
//! [`WIRE_FAMILIES`](rig_core::test_utils::streaming_conformance::WIRE_FAMILIES)
//! must carry a `streaming_conformance_suite!` invocation, or CI fails (#2258,
//! Part III (c) — permanently closes the fixtures-covered-a-subset gap).
//!
//! **Compile-linked, not text-scanned (#2258 F3).** The first version of this
//! registry read the workspace's `.rs` files and matched the `provider: "…"`
//! literal of each `streaming_conformance_suite!` invocation. That made the
//! guard a paper claim: commenting a suite out, or wrapping it in a
//! `#[cfg(any())]`, removed its 11 tests and still matched the text, so the
//! registry reported the family covered. It now reads `SUITE_FAMILIES` consts
//! built out of the `WIRE_FAMILY` const the macro emits *inside each expanded
//! suite*, so a disabled suite is a compile error in the file that hosts it.
//!
//! The remaining seam is binary boundaries: four suites compile into test
//! binaries other than this one and therefore cannot be linked here. They are
//! enumerated in [`OUT_OF_BINARY_FAMILIES`] with the file that expands them, so
//! each is still named somewhere the compiler can be pointed at.

use std::collections::BTreeSet;
use std::path::Path;

use rig_core::test_utils::streaming_conformance::WIRE_FAMILIES;

use super::{streaming_conformance, streaming_conformance_suites};

/// A wire family whose suite compiles into a different test binary than this
/// registry, so it cannot be compile-linked here.
struct OutOfBinaryFamily {
    /// The `WIRE_FAMILIES` entry.
    family: &'static str,
    /// Where the suite is invoked (workspace-relative).
    suite_file: &'static str,
    /// Why it cannot live in the facade's binary.
    reason: &'static str,
}

const OUT_OF_BINARY_FAMILIES: &[OutOfBinaryFamily] = &[
    OutOfBinaryFamily {
        family: "openai_responses_websocket",
        suite_file: "crates/rig-core/tests/integration/streaming_conformance_websocket.rs",
        reason: "drives a real `ResponsesWebSocketSession` over the bundled tungstenite backend \
                 against a local ws server; it is a rig-core integration target behind \
                 rig-core's `tungstenite` feature, which the facade enables only with `websocket`",
    },
    OutOfBinaryFamily {
        family: "candle",
        suite_file: "crates/rig-candle/tests/integration/streaming_conformance.rs",
        reason: "rig-candle is a separate package; the facade does not depend on it",
    },
    OutOfBinaryFamily {
        family: "gemini_grpc",
        suite_file: "crates/rig-gemini-grpc/tests/integration/streaming_conformance.rs",
        reason: "rig-gemini-grpc is a separate package; the facade does not depend on it",
    },
];

/// Union of the suite families compiled into THIS binary.
fn linked_families() -> BTreeSet<&'static str> {
    streaming_conformance_suites::SUITE_FAMILIES
        .iter()
        .chain(streaming_conformance::SUITE_FAMILIES)
        .copied()
        .collect()
}

#[test]
fn all_wire_families_have_conformance_suites() {
    let mut covered = linked_families();
    covered.extend(OUT_OF_BINARY_FAMILIES.iter().map(|entry| entry.family));

    let missing: Vec<&&str> = WIRE_FAMILIES
        .iter()
        .filter(|family| !covered.contains(**family))
        .collect();
    assert!(
        missing.is_empty(),
        "wire families without a streaming_conformance_suite! invocation: {missing:?} \
         (covered: {covered:?})",
    );

    // A family name outside the canonical list is a typo or an unregistered
    // family; both must be fixed at the WIRE_FAMILIES source of truth.
    let unknown: Vec<&&str> = covered
        .iter()
        .filter(|family| !WIRE_FAMILIES.contains(*family))
        .collect();
    assert!(
        unknown.is_empty(),
        "suite invocations name families missing from WIRE_FAMILIES: {unknown:?}",
    );
}

/// An `OUT_OF_BINARY_FAMILIES` entry is an admission that this binary cannot
/// prove the suite exists, so each one must be a genuine gap: a family that IS
/// linkable here belongs in a `SUITE_FAMILIES` const instead, where the
/// compiler enforces it.
#[test]
fn out_of_binary_families_are_genuinely_out_of_binary() {
    let linked = linked_families();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for entry in OUT_OF_BINARY_FAMILIES {
        assert!(
            !linked.contains(entry.family),
            "{} is compile-linked into this binary — drop it from OUT_OF_BINARY_FAMILIES",
            entry.family,
        );
        assert!(
            root.join(entry.suite_file).is_file(),
            "OUT_OF_BINARY_FAMILIES points {} at {}, which does not exist",
            entry.family,
            entry.suite_file,
        );
        assert!(
            !entry.reason.is_empty(),
            "{} needs a reason it cannot live in the facade's binary",
            entry.family,
        );
    }
}
