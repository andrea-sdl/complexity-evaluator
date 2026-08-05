# Agent support

Nothing in this folder installs a skill or enables a hook. Review each file
before you copy or merge it. This `agent` tree is the hand-maintained source
for the generated plugin at `plugins/complexity-evaluator`.

For a host-native install, add the repository marketplace, then install
`complexity-evaluator@complexity-evaluator`:

```sh
codex plugin marketplace add andrea-sdl/complexity-evaluator --ref main
codex plugin add complexity-evaluator@complexity-evaluator

claude plugin marketplace add andrea-sdl/complexity-evaluator
claude plugin install complexity-evaluator@complexity-evaluator
```

The base plugin exposes the explicit-only skill and does not enable hooks. Its
own README gives the local-checkout and opt-in hook steps. Keep using the
manual copy and merge steps below when you do not want a plugin install.

## Use the bundled hook samples

Keep the `agent` folder at the project root. The sample commands use the
bundled checker at `agent/skills/complexity-cli/scripts/check_complexity.py`.
Keep the session working directory at the project root because these commands
use paths relative to it.

Merge one sample into the matching settings file. Keep all existing settings
and hooks.

| Host | POSIX sample | Windows sample | Merge into |
| --- | --- | --- | --- |
| Codex | `agent/hooks/codex.json` | `agent/hooks/codex-windows.json` | `.codex/hooks.json` |
| Claude Code | `agent/hooks/claude.json` | `agent/hooks/claude-windows.json` | `.claude/settings.json` |

The POSIX samples use `python3`. The Windows samples use the Python launcher,
`py -3`. Put the released `complexity` binary on `PATH`, or set
`COMPLEXITY_BIN` in the environment that starts Codex or Claude Code.

Use `/hooks` to review the loaded hooks. Project hooks run only after you trust
the project. Copying this folder alone changes no host settings.

Each user-submitted prompt starts a new file-state baseline. A Stop-hook retry
inside that turn keeps the same baseline.

## Install the explicit skill

To use `$complexity-cli` in Codex, copy `agent/skills/complexity-cli` to
`$HOME/.agents/skills/complexity-cli`. For Claude Code, copy it to
`$HOME/.claude/skills/complexity-cli`.

The skill stays explicit-only. Its Codex metadata sets
`allow_implicit_invocation: false`. Its Claude metadata sets
`disable-model-invocation: true`; the Claude hook samples also set the
`user-invocable-only` override. Installing the skill does not enable a hook.

The installed-skill hook examples in
`agent/skills/complexity-cli/references/hooks.md` use the home install paths
instead of this bundled project path.

## Run the manual Codex eval

The eval asks Codex to refactor four small functions. It needs Node.js 24 or
later, Python 3, PHP, `rustc`, network access, and a Codex login or API key.
In an unpacked release archive, install the pinned Promptfoo development
dependency and use the binary at the archive root:

```sh
cd agent/eval
npm ci --ignore-scripts
npm test
npm run validate
COMPLEXITY_BIN=../../complexity npm run eval
```

In a source checkout, build the release binary and use
`COMPLEXITY_BIN=../../target/release/complexity npm run eval` from
`agent/eval`.

On Windows:

```powershell
cd agent\eval
npm ci --ignore-scripts
npm test
npm run validate
$env:COMPLEXITY_BIN = "..\..\complexity.exe"
npm run eval
```

The short cases in `agent/eval/cases.yaml` cover JavaScript score, TypeScript
control depth, PHP condition predicates, and Rust function span. Each case has
a source file and a separate behavior test. Codex must run the test and real
skill before and after its edit, change only the source, lower the score, meet
all targets, and report the refactor and next useful improvement.

The assertion does not trust the model report. It checks the file change,
reruns the behavior test and CLI, compares the measured metrics, and checks
that the model ran the checker before and after its edit. The runner disables
Promptfoo telemetry, update checks, result sharing, and result caching.

The live eval is manual because it calls Codex. Release CI runs `npm test` and
`npm run validate`; it does not call a model.

Append one case ID to run only that case, for example
`npm run eval -- php-predicates`. Set `COMPLEXITY_EVAL_KEEP_WORKSPACE=1` to
keep a failed source workspace for review. The runner always removes the
temporary Codex home and its copied login data before it keeps that workspace.
