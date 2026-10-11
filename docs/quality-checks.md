# Quality checks

The existing general CI workflow now runs the command panic hunt on every pull request
and push to main:

```sh
cargo test -p photocraft-engine --locked --test panic_hunt -- --include-ignored
```

Including ignored tests is required here: the full registry sweep is deliberately
ignored during ordinary local test runs. The harness tries four parameter shapes per
command and gives each invocation a four-second limit. It reports both escaped panics
and EngineError::Internal returned by the command guard. Normal parameter errors,
disabled commands and successful calls remain valid outcomes.

Worker, dispatch and background-apply panic boundaries use the typed internal error.
Preserve that distinction when adding another panic boundary; reporting a caught panic
as an ordinary parameter failure would make the sweep miss it.

The harness has negative regression tests for caught internal errors. The common
boundary also has a unit regression:

```sh
cargo test -p photocraft-engine --lib jobs::tests::guarded_run
```

This sweep supplements the existing corpus, workspace and rendering checks; use
[Development](development.md) for the full local setup.

After the first published workflow run, select **command panic hunt** as a required
check in branch rules together with the existing required checks.
