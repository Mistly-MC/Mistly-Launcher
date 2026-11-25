# Contributing to Mistly Launcher

Thanks for wanting to help! This repository is the launcher, written in Rust with a [Slint](https://slint.dev/) interface, for Windows and Linux. The in-game mod lives in [Mistly-Client](https://github.com/Mistly-MC/Mistly-Client) and the website in [Mistly-Web](https://github.com/Mistly-MC/Mistly-Web).

By taking part, you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Ways to help

- **Report a bug**: open an issue with what you did, what you expected, what happened, your system (Windows or Linux) and the launcher version (Settings → About). The console's **Send to the team** button gives a code you can paste in the issue.
- **Fix a translation**: the texts are in `crates/misty-core/lang/<lang>.json` (`en.json` is the reference).
- **Fix or add a feature**: for anything bigger than a small fix, open an issue (or ask on [Discord](https://discord.gg/rdQJUgvUh3)) before you start, so that we can agree on the idea first.

## Build and run

You need [Rust](https://www.rust-lang.org/tools/install) (stable). On Linux, the packages listed at the top of `installer/build-linux.sh`.

```bash
git clone --recursive https://github.com/Mistly-MC/Mistly-Launcher.git
cd Mistly-Launcher
cargo run                   # start the launcher
cargo test                  # offline tests, must pass before every pull request
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

The workspace is described in [`crates/README.md`](crates/README.md): `misty-core` holds all the logic without any interface, `misty-ui` the `.slint` files, and `misty-app` the window and the pages' code.

Never commit `curseforge.key`, a token or a password.

## Code style

- Identifiers and comments are in **French**, like the rest of the code. Please keep that style.
- Texts shown in the launcher are never hard-coded: add the key to **the 6 files** in `crates/misty-core/lang/` (in English if you don't speak the language, a translator will follow), then use `loc::t("Key")` in Rust or `Tr.t("Key")` in Slint.
- `misty-core` must never depend on Slint or on a window.
- A new setting goes in `Reglages` with `#[serde(default)]`; never rename or change the type of an existing field.
- Every source file starts with the Mistly header; keep it when you edit a file, and add it to new files.
- The CI runs `cargo fmt --check`, `cargo clippy -D warnings` and `cargo test` on Windows and Linux: a warning fails it.

## Pull requests

1. Fork the repository and create a branch from `main`.
2. Keep each pull request focused on one change.
3. Write commit messages in English, in the imperative, without a final period: `Fix the console filters on Linux`. Add a body that explains *why* when it is not obvious.
4. Add a line to `CHANGELOG.md` for any change players will notice.
5. Describe what you changed and how you tested it, with a screenshot for visual changes.
6. A team member reviews it. Releases go straight to players, so only the team merges and publishes.

## License

Mistly Launcher is distributed under the Misty License 1.0 (`LICENSE.txt`): free to use, modify and share, but not to sell. By submitting a contribution, you agree that it is distributed under this license.
