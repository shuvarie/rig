//! Dependency-graph invariants for the contracts/transport/provider split,
//! which is a feature split inside one package.
//!
//! The contracts compile with no transport and no provider SDK; each provider
//! SDK arrives only with its own feature.

use std::process::Command;

/// `(package, `cargo tree` feature arguments, forbidden dependency names,
/// required dependency names)`. The three lists are space-separated; empty
/// features select the package's defaults.
type Graph = (&'static str, &'static str, &'static str, &'static str);

const GRAPHS: &[Graph] = &[
    // Contracts alone: no transport, no async runtime, no provider SDK.
    ("rig", "--no-default-features", "tokio reqwest", ""),
    // Each provider SDK is reachable only through its own feature.
    (
        "rig",
        "--no-default-features --features reqwest",
        "bedrock candle fastembed gemini-grpc vertexai rmcp",
        "reqwest tokio",
    ),
    (
        "rig",
        "--no-default-features --features bedrock",
        "candle fastembed vertexai gemini-grpc rmcp",
        "aws-sdk-bedrockruntime",
    ),
    (
        "rig",
        "--no-default-features --features candle",
        "bedrock fastembed vertexai gemini-grpc rmcp",
        "candle-core",
    ),
];

/// `cargo` with `args`, run from the workspace root; stdout on success.
fn cargo_stdout(args: &[&str]) -> String {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(args)
        .output()
        .expect("cargo runs");
    assert!(
        output.status.success(),
        "cargo {} failed:\n{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("cargo output is utf-8")
}

/// `cargo tree -e normal --prefix none` for `package` under `features`, as the
/// package names in the resolved graph.
fn normal_dependency_names(package: &str, features: &str) -> Vec<String> {
    let mut args = vec![
        "tree", "--locked", "-p", package, "-e", "normal", "--target", "all", "--prefix", "none",
    ];
    args.extend(features.split_whitespace());
    cargo_stdout(&args)
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_owned)
        .collect()
}

#[test]
fn crate_boundaries_hold_in_the_resolved_dependency_graph() {
    for (package, features, forbidden, required) in GRAPHS {
        let names = normal_dependency_names(package, features);
        let selection = if features.is_empty() {
            "default features"
        } else {
            features
        };
        for crate_name in forbidden.split_whitespace() {
            assert!(
                !names.iter().any(|name| name == crate_name),
                "`{package}` ({selection}) must not depend on `{crate_name}` through its normal dependencies"
            );
        }
        for crate_name in required.split_whitespace() {
            assert!(
                names.iter().any(|name| name == crate_name),
                "`{package}` ({selection}) must depend on `{crate_name}`"
            );
        }
    }
}
