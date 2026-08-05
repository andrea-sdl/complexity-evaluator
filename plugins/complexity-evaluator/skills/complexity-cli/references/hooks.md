# Optional Stop hooks

These hooks are examples only. The skill and sync scripts do not enable them.

Use `UserPromptSubmit` to record the task's starting Git and file state. Use
`Stop` to check supported files that changed after that baseline. This catches
edits made through shell commands and changes committed before Stop.

The checker filters unsupported files before it starts `complexity`. If a task
changes only unsupported files, the hook returns no decision and stays silent.
For mixed changes, only supported files reach the CLI. An unchanged supported
file that was already dirty before the prompt is outside the task scope.

If the baseline hook is not configured or its state is missing, Stop falls
back to all Git-changed supported files. A `PostToolUse` hook would run after
each edit and add avoidable work.

The checker asks the agent to continue on `REVISE`, `FAIL`, or `BLOCKED`. It
keeps the original baseline during that continuation. It allows the next stop
when `stop_hook_active` is true, which prevents an endless hook loop.
Each later user-submitted prompt starts a new baseline, even when the prior
Stop hook blocked.

Before enabling a hook, install `complexity` in the default Cargo bin, put it
on `PATH`, or set `COMPLEXITY_BIN` in the environment that starts Codex or
Claude Code.

## Scope decision

Keep the core CLI strict for unsupported explicit files. The hook wrapper owns
file filtering because it knows the task lifecycle and Git state. This keeps
manual CLI mistakes visible without making AI hooks noisy.

Scope is file-based. The CLI still reports old complex functions inside a file
that the task changed. The agent must separate those old findings from changes
made for the task. Function-level diff tracking is outside this small hook.

## Codex

Merge this into the repository's `.codex/hooks.json`:

```json
{
  "description": "Check changed JS, TS, PHP, Rust, and Python before Codex stops.",
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "python3 \"$HOME/.agents/skills/complexity-cli/scripts/check_complexity.py\" --baseline-hook",
            "timeout": 30
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "python3 \"$HOME/.agents/skills/complexity-cli/scripts/check_complexity.py\" --hook",
            "timeout": 120,
            "statusMessage": "Checking code complexity"
          }
        ]
      }
    ]
  }
}
```

Project hooks run only in a trusted project. Review and trust the exact hook in
Codex with `/hooks`.

## Claude Code

Merge this into the repository's `.claude/settings.json`:

```json
{
  "skillOverrides": {
    "complexity-cli": "user-invocable-only"
  },
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "python3 \"$HOME/.claude/skills/complexity-cli/scripts/check_complexity.py\" --baseline-hook",
            "timeout": 30
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "python3 \"$HOME/.claude/skills/complexity-cli/scripts/check_complexity.py\" --hook",
            "timeout": 120,
            "statusMessage": "Checking code complexity"
          }
        ]
      }
    ]
  }
}
```

The `user-invocable-only` override keeps the skill out of Claude's model
context while leaving `/complexity-cli` available. Use `/hooks` to inspect the
loaded hook.
