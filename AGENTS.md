# Repository Guidelines

## Project Structure & Module Organization
CLI code is in `src/`; `main.rs` hands off to `lib.rs`, with `ui/` for screens and `command/` for git actions plus unit tests. Helpers live in `util/`. Update `spec/` documents before altering code so UI expectations stay clear. Integration flows live in `tests/integration`; git fixtures in `tests/git_test`. Release tooling sits under `scripts/`.

## Build, Test, and Development Commands
- `cargo build` — compile the binary; add `--release` before tagging a build.
- `cargo test` — run unit and integration suites; add `-- --nocapture` when debugging UI logic.
- `cargo fmt` — apply standard Rust formatting before commits.
- `cargo clippy -- -D warnings` — lint with deny-on-warning parity with CI.
- `scripts/release.sh <version>` — convenience script to bump, tag, and publish.
- Avoid `cargo run`; run the installed binary instead to prevent hung sessions.

## Coding Style & Naming Conventions
Follow Rust 2024 defaults: four-space indentation, `snake_case` for modules and functions, `UpperCamelCase` for types, and `SCREAMING_SNAKE_CASE` for constants. Keep UI state structs tight; move shared helpers into `util/` instead of duplicating logic. Document tricky control flow with brief comments consistent with `src/ui/`, and run `cargo fmt` plus `cargo clippy` before opening a pull request.

## Testing Guidelines
Unit tests live beside command implementations (files ending in `_test.rs`), and integration coverage belongs in `tests/integration`. Prefer fixtures built with `tempfile` and `serial_test` to isolate git state. Extend `tests/git_test` when touching low-level git behavior. New UI flows should update the relevant spec markdown first and add a regression test that fails without the change.

## Commit & Pull Request Guidelines
Commit messages follow a Conventional Commit flavor (`feature:`, `fix:`, `test:`, `chore(release):`, etc.); keep the subject under ~60 characters and add context in the body when behavior changes. Pull requests should describe the user-facing impact, list validation steps (commands run plus screenshots or gifs for UI tweaks), and reference related issues. Confirm clippy, fmt, and tests pass locally, and call out skipped coverage so reviewers can weigh the risk.

## Documentation & Release Notes
When modifying workflows or key bindings, update both the relevant files in `spec/` before touching code and any user-facing docs (`README.md`, `GEMINI.md`). Before a release, ensure `scripts/release.sh` generates the expected changelog entry and that README install steps stay accurate.

## Architecture Notes
The TUI relies on `pancurses`; avoid adding `git2` and keep specs in sync with UI behavior updates before shipping changes.
