# Complexity Evaluator plugin

This plugin exposes `$complexity-cli` only when you invoke it. Install the
`complexity` binary separately. A missing binary reports `BLOCKED`.

## Install for Codex

```sh
codex plugin marketplace add andrea-sdl/complexity-evaluator --ref main
codex plugin add complexity-evaluator@complexity-evaluator
```

Start a new Codex session, then invoke `$complexity-cli`.

## Install for Claude Code

```sh
claude plugin marketplace add andrea-sdl/complexity-evaluator
claude plugin install complexity-evaluator@complexity-evaluator
```

Run `/reload-plugins` or start a new session, then invoke
`/complexity-evaluator:complexity-cli`.

## Optional hooks

The base plugin does not enable hooks. In a source checkout or unpacked plugin,
copy one matching sample before you add the local marketplace or reload the
plugin:

```sh
cp hooks/codex.json hooks/hooks.json
```

Choose only one of `codex.json`, `codex-windows.json`, `claude.json`, or
`claude-windows.json`. On Windows, use `copy` instead of `cp`. The samples use
the host-provided `CLAUDE_PLUGIN_ROOT` variable and keep the checked repository
as the working directory.

Do not hand-edit generated files. Edit `agent/` in the source repository and
run `python3 release/sync_plugins.py`.
