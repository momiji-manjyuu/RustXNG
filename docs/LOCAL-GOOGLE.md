# Local Google compatibility branch

This checkout follows upstream `main` at `d364e3b` (version 0.27.1), with the seven
Google adapters from the previous local checkout, `dc0e6c0`, restored. The package
version is `0.27.1+google.local.1` so the running binary can be distinguished from an
official release. Nothing has been pushed upstream.

The working branch is `local/google-preserved`. The previous checkout remains at
`backup/pre-google-update-20261103` and `main`.

## Preserved behavior

The adapters, factory registrations, engine names, shortcuts and settings are kept.
Google web, Google News, Google Play and Google Images retain their enabled defaults.
Google Scholar, Google Videos and Google CSE retain their disabled defaults. CSE
still requires its configured token. Existing user settings continue to merge with
the bundled defaults. Other upstream provider removals and security fixes remain.

The other six Google adapters retain their original endpoint and parser behavior.
Google web has the compact-layout compatibility fix described below. Registration
and offline fixtures do not guarantee that a provider will accept live requests.

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

## Initial migration validation on 2026-11-03

Windows x64 with Rust 1.99.1: formatting, Clippy with warnings denied, all-target
tests (937 passed; one external-MySQL test ignored), and the three agent-setup smoke
cases passed. Limit build parallelism on this machine: unrestricted concurrent
debug builds exhausted Windows virtual memory during the first attempt.

With the existing installed user settings, live Google News and Wikipedia searches
returned results. During the initial migration, Google web returned an HTTP-200
JavaScript bootstrap page that the old parser reported as access denied.

## Google web compatibility fix

The cause was reproduced using the server's own `Network` implementation. A compact
HTML request on `/search` returns result markup without executing JavaScript.
The request format uses the same compact-layout User-Agent as the retained Google
Images adapter. `/wml/search` returned 403 in this environment, so this adapter
keeps `/search`. No browser process, profile, extra service or dependency is used.

The parser now reads compact result cards, their title links and snippets, while
retaining the old desktop selectors. UI locale sets `hl`; only an explicit search
language sets `lr`, so automatic searches are not limited by the UI language.
Paging, time range and SafeSearch parameters remain supported.

JavaScript-only pages, CAPTCHA responses, HTTP rejection and unknown markup are
reported separately. Challenges are not solved or followed automatically.
The request/layout fields were checked against the
[SearXNG Google implementation](https://github.com/searxng/searxng/blob/master/searx/engines/google.py)
and real responses from this machine. The first successful server-stack probe
returned 11 parsed results; subsequent upstream restrictions remain possible.


Verification of `0.27.1+google.local.1`: 940 all-target tests passed, with one
external-MySQL test ignored. Formatting, Clippy with warnings denied, and all three
release-binary agent setup smoke cases passed. With the installed user settings
on a temporary loopback port, English/Japanese Google searches, the `!go` shortcut
and Google News each returned five results without engine errors.
