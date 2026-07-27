# complexity agent rules

Read [README.md](README.md), [SPEC.md](SPEC.md), [DESIGN.md](DESIGN.md), and
[TASKS.md](TASKS.md) before you change this project.

1. Claim a `ready` task in `TASKS.md`. Record the owner, scope, and status.
   Work in files and behavior that do not overlap another active task.
2. Use TDD for public behavior. Add one failing public test, make it pass, and
   then refactor only if the result becomes easier to read.
3. Update `SPEC.md`, `DESIGN.md`, and `TASKS.md` for each public contract or
   design change.
4. Preserve all `core-v1` scores, contributions, ranges, and parser error
   rules. Do not change a score without an approved fixture that shows why.
5. Ask for approval before you add a production dependency.
6. Keep the two parser engines separate. Share only the CLI, discovery, report,
   sort, format, summary, and exit flow described in `DESIGN.md`.
7. Before handoff, run:

   ```sh
   cargo fmt --all -- --check
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --all-targets
   cargo build --release
   ```

8. Record the checks and results in `TASKS.md`. Do not mark work `done` when a
   required check did not run or did not pass.
