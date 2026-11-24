# Releasing

*Português: [RELEASING.md](RELEASING.md).*

How to publish an ArqRustUI version on crates.io.

## GitHub repository preparation

Use the **complete `ArqRustUI` directory as the repository root**, with its existing
Git history. Include `src/`, `crates/`, `assets/`, `examples/`, `tests/`, `docs/`,
`.github/`, the manifests, lockfile, licenses and Git configuration files. The
parent ArqRust suite and the PocketTools application are independent projects.
Do not upload only `src/`: the main package needs the local icon crates.

The root [README.md](../README.md) is in English and uses screenshots stored in
`docs/screenshots/`. The main package's `readme` metadata points to this file.
Keep screenshot paths relative and keep icon provenance and license notices.
Build output and local caches are ignored; original work archives have been
removed from the source directory and retained outside the repository.

Review `git status` before staging. This checkout contains earlier source changes
and untracked first-party icon packages; keep those packages when publishing the
repository. No cleanup step should erase `.git/`, rewrite history or discard
existing code changes. Git transmits repository history; `.git/` is not a folder
to add manually through GitHub's file-upload interface.

After creating the intended GitHub repository, configure its actual remote URL
and add that verified URL to package `repository` metadata. No URL is invented
in advance. A GitHub source upload does not publish anything to crates.io.

Local preflight, from the library directory:

```powershell
cargo fmt --all --check
cargo clippy --offline --workspace --all-targets -- -D warnings
cargo test --offline --doc
cargo build --offline --release --examples
cargo metadata --offline --format-version 1
cargo package --offline --workspace --allow-dirty --no-verify
```

`--allow-dirty` is for a local review before committing. `--no-verify` checks
packaging without rebuilding the extracted packages; it is not a replacement
for release verification. Inspect metadata to ensure all packages are local
first-party code. Offline mode alone is not a dependency provenance check.

The recorded Windows suite still has known failures (see the README and feature
matrix). Resolve them and verify the intended release commit before publishing
a release. Package assembly alone does not certify platform functionality.

## The ten crates

| Crate | Folder | Contents |
|---|---|---|
| `arqrustui` | `./` | the library; the `icons` feature (default) depends on `arqrustui-icons` |
| `arqrustui-icons` | `crates/arqrustui-icons/` | facade: re-exports the eight data crates |
| `arqrustui-icons-data1` … `-data8` | `crates/arqrustui-icons-dataN/` | the icon bytes (`.ico`, `.png`, `.svg`), up to 9 MB each |

crates.io accepts at most 10 MiB per compressed package, and the icon pack is about
66 MB; that is why it comes in parts. The ten crates **always share the same
version** and are published together, in this order: the eight data crates, the
facade and, last, `arqrustui` (each one can only go after the ones it uses).

## Before publishing

1. The commit's CI is green, including the "Pacotes do crates.io (tamanho)" (crates.io package size) job.
2. Change the version in the root `Cargo.toml`: in `[workspace.package]` and in the
   nine entries of `[workspace.dependencies]` (that is the only place; the crates in
   `crates/` inherit the version with `version.workspace = true`).
3. In `CHANGELOG.md`, rename `[Não publicado]` to the version with the date and open
   an empty `[Não publicado]` above it; do the same in `CHANGELOG.en.md` with
   `[Unreleased]`.
4. Check the packages locally: `cargo package --workspace` packages and builds
   each crate from its `.crate` file, resolving the not-yet-published crates
   against each other.

## Publish

With a Cargo version whose `cargo publish --help` supports `--workspace`, one
command publishes all ten in dependency order:

```powershell
cargo publish --workspace
```

Otherwise, publish them individually in dependency order:

```powershell
foreach ($n in 1..8) { cargo publish -p "arqrustui-icons-data$n" }
cargo publish -p arqrustui-icons
cargo publish -p arqrustui
```

Then create the `vX.Y.Z` tag on the published commit.

## Adding icons after the first release

An icon never moves to another data crate (the constant name has to keep resolving
in the same facade). New icons go into the last data crate while it has room (the
CI size job warns); when they do not fit, create `arqrustui-icons-data9` (copy the
structure of an existing one) and add it as a dependency and a `pub use` in
`crates/arqrustui-icons`. The step-by-step guide is in "Acrescentar um ícone"
(Adding an icon), in [`assets/icons/README.md`](../assets/icons/README.md)
(Portuguese).
