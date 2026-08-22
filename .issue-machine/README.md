# Recipe issue machine

This branch is the persistent reboot-safe machine snapshot. Create its worktree
at the configured path and start the tmux session:

```text
git worktree prune
git worktree add /home/nate/Desktop/recipe-issue-machine machine
tmux new-session -d -s recipe-issue-machine /home/nate/Desktop/recipe-issue-machine/.issue-machine/session
tmux attach -t recipe-issue-machine
```

The current unfinished observation is CPU cursor 114. It reproducibly exits
with SIGSEGV inside generated `artifact.so`. The concurrent coordinator must
queue stable process-level failures instead of terminating. Cursor 108 and the
pending review queue remain saved in `machine.toml` and `queue.ogdl`.

This machine continuously runs Recipe's exhaustive Cartesian traversal. It
records every cursor result immediately and appends stable failures to one
durable queue before advancing the cursor. Reviewers traverse that queue while
cursor discovery continues. Each model uses its configured concurrency limit,
while different models run concurrently. The first available
classifier returns one schema-validated decision: create a
`bug` issue or comment on an existing issue.

One resolver runs independently of discovery and review. It selects the first
open issue that no open pull request closes, then starts
`claude-fable-5` at high effort with unrestricted Claude Code permissions. The
initial `/goal` requires the session to read that issue, reproduce it, implement
the root fix in a separate worktree based on current `origin/minimal`, validate
the public path, and create one pull request with `Fixes #<issue>`. The resolver
does not release its only slot until that pull request exists.

Independent discovery workers claim disjoint cursors from one allocator. The
default configuration runs one worker on `amd0` and one worker through
`RECIPE_FORCE_CPU=1`. Results can finish out of order, but `machine.toml`
advances only across the contiguous completed cursor frontier. A crash can
repeat unfinished work but cannot skip it. Every terminal record and failure
packet identifies the backend that executed the cursor.

Each traversal process has the configured `trial_memory_mib` address-space
limit. A composition that exceeds that limit fails through the same public
process boundary and enters the normal review queue instead of exhausting the
host and terminating the machine.

The machine runs each model up to its configured concurrency limit. Ox Alpha,
whose current catalog key is `opencode/x-preview-f-free`, runs up to 20
requests concurrently. Every other OpenCode model runs one request at a time.
Each live
reviewer uses the `provider/model` identity shown in the terminal and log. The
configured providers and model order are:

- `codex/gpt-5.3-codex-spark`
- `kimi/kimi-code/k3`
- Every configured `opencode/<free-model>` concurrently
- `copilot/auto`, updated to the model selected by Copilot routing
- `claude/sonnet-5-max`
- `ollama/minimax-m3:cloud`
- `agy/<model>` in configured order
- `deepseek/deepseek-v4-pro`

The machine uses each provider until that provider is unavailable. It does not
wait for consensus. If every route is unavailable, the reviewer retains the
failure at the head of the queue, waits for `provider_poll_seconds`, and tries
again while cursor discovery continues.

## Classifier access

Every classifier can read the Recipe repository and call only two GitHub tools:

- `search_issues` searches the complete Recipe issue history and returns issue
  numbers, titles, states, and URLs.
- `read_issue` returns one complete issue, including every comment.

The classifiers search issues on demand and read every plausible match in full.
The machine does not preload, cache, summarize, or truncate the issue tree.
Codex, OpenCode, Claude, Ollama, and GitHub Copilot use project-scoped MCP
configurations. Kimi uses `/home/nate/.kimi-code/mcp.json`. Agy uses the imported
`recipe-issue-reader` plugin and a pre-tool hook that rejects mutation tools.

The classifier tool surfaces contain only repository readers and the two issue
readers. A classifier cannot edit files, run shell commands, delegate work,
create branches, commit, push, publish issues, or open pull requests.

Each provider keeps its real conversation or session identifier. If the final
assistant response does not match `decision.schema.json`, the machine asks that
same conversation to return a corrected JSON object. Authentication failures,
usage-limit failures, and unavailable models fall through without a repair
turn.

## Publication

The Rust machine owns the only GitHub write boundary. For a `new` decision, it
creates an issue with the `bug` label. For a `comment` decision, it comments on
the selected issue. Every published issue or comment records:

- The traversal failure fingerprint
- The exact classifier provider, model, and effort
- The complete structured classifier decision
- The deterministic failure packet and public Recipe reproduction

GitHub assignees remain reserved for people who own the resulting work.

Set `publish = false` to perform real traversal and classification without a
GitHub write. The machine then leaves the cursor at the failed composition and
stops. Set `batches = 0` for continuous traversal.

## Observability

The terminal keeps completed cursor results above a live tree:

```text
cpu   cursor 572     time  13.3093 s  status PASS
queued
└─ 69 reviews
live
├─ trial
│  ├─ amd0  cursor 571     time  50.8573 s
│  └─ cpu   cursor 575     time  20.1335 s
├─ review
   ├─ opencode
   │  ├─ big-pickle                    cursor 561  time 51.0895 s
   │  └─ hy3-free                      cursor 568  time 42.0012 s
   ├─ copilot/claude-haiku-4.5         cursor 570  time 12.0031 s
   ├─ claude/sonnet-5-max              cursor 573  time 10.1011 s
   └─ agy/claude-opus-4-6-thinking     cursor 536  time 51.0867 s
└─ resolve
   └─ claude/fable-5-high              issue #171  time 51.1012 s
```

The terminal groups a provider under one node only while multiple models from
that provider are active. A provider with one active model stays on one line.

A failed cursor then contains two classification lifecycle records:

```text
CLASSIFY model=<provider/model> composition=<composition>
ISSUE model=<provider/model> <action> issue=#<number> url=<url>
RESOLVE model=claude/fable-5-high issue=#<number> url=<issue-url>
PR model=claude/fable-5-high issue=#<number> url=<pr-url>
```

An unpublished trial ends with `DECISION`. Unavailable models and provider
fallthrough do not print to the terminal.

The machine writes the same lifecycle records to the repository-root
`recipe.log`. Set the single `debug` option to `true` to add provider failures,
polling, and structured-output repair details to that file. There are no other
logs or verbosity controls.

## Run the machine

Build and run the machine from this directory:

```text
rustc --edition 2021 run.rs -o run
./run
```

Configuration is in `machine.toml`. The Recipe repository is a detached
worktree, so the traversal cannot alter Nate's active checkout and does not
create a branch. `machine.toml` stores the resumable cursor. GitHub issues and
comments remain the authoritative deduplication state. `queue.ogdl` is the
durable pending-review state. A packet remains there until its decision is
published, and a restart resumes the oldest queued packet first.
