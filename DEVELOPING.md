# Development verification

The workflow is minimal relevant local checks, independent review, prompt
authorized publication, and comprehensive GitHub CI. Broad local verification is
not a prerequisite for creating or updating a PR.

Before publication, inspect the intended diff against its actual merge base,
preserve unrelated changes, check formatting/whitespace as applicable, and run
the smallest useful check for the changed behavior. A focused regression test or
narrow compile check can suffice for code. For documentation or instructions,
review the diff and links/consistency; do not run Rust compilation, full docs, or
workspace tests merely because instructions changed.

Workspace-wide tests, all-features builds, every provider/example, full
docs/doctests, the WASM checks and the Docker suites belong in CI by default.
Explain the concrete need before an expensive local check; do not invent
hypothetical risks to justify a full local gate. Install only the prerequisites
the small local checks you choose need. Iterate on one package with the `local`
nextest profile ([Local builds](#local-builds)).

## Commands CI runs

CI is a set of direct Cargo commands, with no planner and no check registry. The
subset worth running before a push is:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
```

The remaining lanes are `cargo test --workspace --doc`, `cargo doc --workspace
--no-deps` with `RUSTDOCFLAGS=-D warnings`, and per-package `cargo check
--target wasm32-unknown-unknown` for `rig-core`, `rig-agent` and `rig`.

## Choosing a focused check

| Change | Useful local evidence |
| --- | --- |
| Documentation or agent instructions only | Full diff, relative links, and consistency with the referenced code/configuration; no Rust build |
| Test implementation | Run the affected tests; use package-scoped Clippy for new lint-sensitive patterns |
| Public API or feature boundary | Focused behavior test and compilation of known consumers in the affected configuration |
| Target-specific implementation | Inspect both branches; check the affected target when available, and distinguish source review from executed checks |
| CI failure | Read the failed job's diagnostics, fix the cause, then run the narrow owning command |

These are selection guidelines, not a checklist to run in full. Command recipes
and test-specific pitfalls live in [tests/README.md](tests/README.md#core-tests).
Serialize Cargo commands sharing a target directory unless they can make real
progress independently; concurrent commands often just wait for the build lock.

## Reading CI failures

Start with job names and status on the current PR head, then inspect only
failed-job diagnostics and enough surrounding context to understand the failure.
Retain the fetched log outside the repository; search it instead of repeatedly
fetching or printing the entire workflow log.

If `gh run view --log-failed` cannot read a completed job while its workflow is
still running, fetch that job directly:

```sh
gh api repos/0xPlaygrounds/rig/actions/jobs/JOB_ID/logs
```

Capture the response before extracting diagnostics; replace `JOB_ID` with the
failed job's ID. A truncated log is not evidence that later errors are absent.

## Local builds

- `cargo nextest run --locked --profile local -p <crate> [filter]` is the inner
  loop. The `local` profile has no retries, stops at the first failure, and
  skips the service suites and the trybuild cases (`--ignore-default-filter`
  runs them).
- The dev profile keeps line tables for workspace code and no debuginfo for
  dependencies. For a debugger, add `--config 'profile.dev.debug=true' --config
  'profile.dev.package."*".debug=true'`; `CARGO_PROFILE_DEV_DEBUG` alone does
  not reach dependencies.
- Keep incremental compilation on; CI sets `CARGO_INCREMENTAL=0` because its
  runners start fresh.
- Worktrees of this repository can share one target directory: set
  `build.target-dir` in a `.cargo/config.toml` above them. Dependencies then
  build once instead of once per worktree; builds that run at the same time
  wait on the directory's lock.
- Give rust-analyzer its own target directory
  (`rust-analyzer.cargo.targetDir: true`) so it does not hold that lock while
  you build.
- On macOS, add your terminal to System Settings > Privacy & Security >
  Developer Tools; otherwise each new test binary is scanned on its first run.

## Prerequisites

The repository toolchain (`rust-toolchain.toml`), nextest (0.9.130 or newer, for
the profile inheritance in `.config/nextest.toml` and the thread count it gives
tests), Clippy, rustfmt, protoc, and Docker for the storage suites.

## Hosted CI

`ci.yaml` runs on every pull request, merge group and manual dispatch: `fmt`,
`clippy`, `test`, `doctest`, `doc` and `wasm`, each a single Cargo command.
`cd.yaml` runs the same workflow on every push to `main` and then release-plz.
Tokens are read-only except release-plz. No job receives provider secrets: live
provider tests are `#[ignore]`.

Each package links its integration tests into one binary, `integration`, with one
module per suite (see [tests/README.md](tests/README.md)). Recorded provider
replies live in `tests/fixtures/cassettes/` and are read only.

## Refreshing the model catalog

The catalog is the data file `crates/rig-core/src/catalog/models.json`; the
schema is `crates/rig-core/TYPED_OPTIONS.md` section 4. The `cargo xtask catalog
sync` tool that regenerated it from models.dev was removed with the rest of
`xtask`; it is recoverable from the `fork-point` tag (`xtask/src/catalog.rs`)
while this fork still carries the multi-crate tooling. Until it returns, edit
the JSON directly and review the result as data.
