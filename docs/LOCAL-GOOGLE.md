# Local Google compatibility branch

This checkout follows upstream `main` at `d364e3b` (version 0.27.1), with the seven
Google adapters from the previous local checkout, `dc0e6c0`, restored. The package
version is `0.27.1+google.local` so the running binary can be distinguished from an
official release. Nothing has been pushed upstream.

The working branch is `local/google-preserved`. The previous checkout remains at
`backup/pre-google-update-20261103` and `main`.

## Preserved behavior

The adapters, factory registrations, engine names, shortcuts and settings are kept.
Google web, Google News, Google Play and Google Images retain their enabled defaults.
Google Scholar, Google Videos and Google CSE retain their disabled defaults. CSE
still requires its configured token. Existing user settings continue to merge with
the bundled defaults. Other upstream provider removals and security fixes remain.

The restored adapters keep their old endpoint and parser behavior. Registration and
offline fixtures do not guarantee that a provider will accept live requests.

## Following future upstream updates

Keep local work committed on this branch, then fetch and merge upstream:

```powershell
git switch local/google-preserved
git fetch origin
git merge origin/main
```

Resolve any conflicts while preserving the seven `src/engines/google*.rs` files,
their module/factory entries and the local section of `settings.toml`. When upstream
changes the package version, retain `+google.local` in both `Cargo.toml` and the
`rustxng` entry of `Cargo.lock`. Refresh this note's upstream revision as needed.

```powershell
python tools/provider_inventory.py
cargo fmt --all --check
cargo test --locked --all-targets -j1
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release --bin rustxng -j2
```

Validate a new build on a temporary loopback port with the existing user settings
before replacing the installed executable. Preserve the old executable and settings
for rollback. Do not use `rustxng --update` for this branch: it installs an official
binary, which does not contain these local adapters. Build future updates from this
branch instead.

## Validation on 2026-11-03

Windows x64 with Rust 1.99.1: formatting, Clippy with warnings denied, all-target
tests (937 passed; one external-MySQL test ignored), and the three agent-setup smoke
cases passed. Limit build parallelism on this machine: unrestricted concurrent
debug builds exhausted Windows virtual memory during the first attempt.

With the existing installed user settings, live Google News and Wikipedia searches
returned results. Google web returned access denied both before and after the
update; preserving its adapter does not remove this upstream restriction.
