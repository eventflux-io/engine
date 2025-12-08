# Repository Guidelines

## Project Structure & Module Organization
- Core lives in `src/` (`core/` runtime, `query_api/` AST, `sql_compiler/` parser).
- Integration tests in `tests/`; examples in `examples/`; benches in `benches/`; proto defs in `proto/`; docs/plans in `docs/` and `feat/`.
- Build artifacts land in `target/`; never commit them.

## Build, Test, and Development Commands
- `cargo build` / `cargo build --release`: debug/opt builds.
- `cargo check`: fast compile pass.
- `cargo fmt` then `cargo clippy`: format/lint; run before PRs.
- `cargo test` (or `-- --nocapture`); add perf flag only for perf work.
- `docker-compose up -d` then `cargo test redis`: exercise Redis-backed paths.
- `cargo bench`: only when touching hot paths.

## Coding Style & Naming Conventions
- Rust 2021 idioms; let `cargo fmt`/`cargo clippy` define layout and lint.
- Functions/modules/files: `snake_case`; types/traits: `CamelCase`; constants: `SCREAMING_SNAKE_CASE`.
- Prefer small modules under `core/`; keep parser changes scoped to `sql_compiler/`.

## Testing Guidelines
- Add unit tests alongside new logic; put SQL/runtime coverage in `tests/`.
- Name tests after behavior (`handles_empty_window`, `redis_persistence_roundtrip`); include regressions for bug fixes.
- Document unusual setups in test comments, not in code.

## Commit & Pull Request Guidelines
- Commits: single-line <60 chars, imperative, capitalized; never mention AI or co-author tags.
- PRs: describe intent/approach, link issues, list test commands, add screenshots/output for user-facing changes.
- Keep changes minimal per PR; run `cargo fmt && cargo clippy && cargo test` before requesting review.

## Feature Docs in `feat/`
- New feature work starts by creating `feat/<feature>/` with one Markdown doc; keep it the single source of design decisions and code deltas.
- Before coding or refactoring, read relevant `feat/` docs and update them as you go so others need not rescan the tree.
- Always add tests; assertions must be meaningful, accurate, non-duplicative, and aligned to the feature doc.

## Configuration & Security Notes
- Config: SQL WITH clauses > stream-level TOML > application TOML defaults > Rust defaults. Keep secrets in env vars referenced from TOML (e.g., `${KAFKA_PASSWORD}`).
- When testing extensions, prefer dynamic libs via `--extension`; do not commit generated binaries or secrets.

## Safety, Refactoring, and Priorities
- Never use `sed`/`awk` or bulk deletion for refactors; take backups before large edits and get explicit approval before deleting code.
- After file changes, verify with `cargo build` and targeted `cargo test`.
- For priorities or grammar/disabled-test work, consult `ROADMAP.md` first, then `MILESTONES.md`; avoid guessing.
