# Recipe issue machine

This branch is the persistent, reboot-safe machine snapshot. Create its
worktree, install the user service, and attach to its tmux session:

```text
git worktree prune
git worktree add /home/nate/Desktop/recipe-issue-machine machine
paru -S deepseek-harness-bin
rustc --edition 2024 /home/nate/Desktop/recipe-issue-machine/.issue-machine/issue-reader.rs -o /home/nate/Desktop/recipe-issue-machine/.issue-machine/issue-reader
rustc --edition 2024 /home/nate/Desktop/recipe-issue-machine/.issue-machine/run.rs -o /home/nate/.local/bin/recipe-machine
dsh plugin --profile recipe-machine add file:/home/nate/Desktop/recipe-issue-machine/.issue-machine/dsh
install -Dm644 .issue-machine/recipe-issue-machine.service /home/nate/.config/systemd/user/recipe-issue-machine.service
systemctl --user daemon-reload
systemctl --user enable --now recipe-issue-machine.service
tmux attach -t recipe-issue-machine
```

This machine continuously runs Recipe's exhaustive Cartesian traversal. It
records every cursor result immediately and appends stable failures to one
durable queue before advancing the cursor. Reviewers traverse that queue while
cursor discovery continues. Each model uses its configured concurrency limit,
while different models run concurrently. The first available
classifier returns one schema-validated decision: create a
`bug` issue or comment on an existing issue.

The tmux session executes the coordinator as its final process. If the
coordinator exits, tmux exits with it, and the user service restarts the whole
machine instead of leaving an interactive shell that reports a false healthy
state.

The resolver pool has a ceiling of 20 workers and a separate process-memory
budget. The current 80 GiB admission budget permits all 20 resolver process
trees, and each tree has a 4 GiB memory limit. Resolver sessions share one Cargo
target directory, compile one Cargo job at a time, and omit dev and test debug
symbols. This prevents concurrent worktrees from linking duplicate debug
artifacts while all 20 model sessions remain active. Each active resolver
claims a different open issue that no open pull
request closes, then starts
`claude-fable-5` at high effort with unrestricted Claude Code permissions. The
initial `/goal` requires the session to read that issue, reproduce it, implement
the root fix in a separate worktree based on current `origin/minimal`, validate
the public path, and create one pull request with `Fixes #<issue>`. Each resolver
uses the configured memory limit and keeps its issue claim until it finishes.
A failed resolver releases its issue, waits for the configured poll interval,
and returns to issue selection instead of terminating its worker. Resolver
usage-limit failures enter the same model cooldown used by reviewers. A
reported epoch reset keeps every resolver worker idle until that time, while an
unparseable reset disables the resolver model for the rest of the machine run.
Normal output omits the provider transcript, and the single `debug` option adds
it to `recipe.log` when diagnostics are required. Resolver
admission counts active `recipe-resolve-*` systemd units, so restarting the
machine cannot duplicate an issue claim or exceed the memory budget.
The terminal derives active resolver rows and elapsed times from those same
systemd units, so coordinator restarts do not hide work already in progress.

Independent discovery workers claim disjoint cursors from one allocator. The
default configuration runs one worker on `amd0` and one worker through
`RECIPE_FORCE_CPU=1`. Results can finish out of order, but `machine.toml`
advances only across the contiguous completed cursor frontier. A crash can
repeat unfinished work but cannot skip it. Every terminal record and failure
packet identifies the backend that executed the cursor.

Each traversal process has the configured `trial_memory_mib` address-space
limit. The current 8 GiB limit leaves enough virtual address space for ROCm
queue creation. If a traversal terminates by signal, the machine reruns the
same cursor through the same bounded public entrypoint and queues the crash only
when that replay also terminates by signal. A transient process death uses the
replay result and does not enter the review queue.

Generated reproductions use a process-specific saved model path. Concurrent
resolvers can exercise save, resume, and inference without replacing another
reproduction's model file. Each failure packet embeds the complete reproduction
source before the machine removes the completed temporary script, so continuous
traversal does not retain one file per cursor.

The machine derives one fixed review-worker pool from the configured route
limits. Queue growth does not create additional operating-system threads. Each
live reviewer uses the `provider/model` identity shown in the terminal and log.
The configured providers are every `opencode/<free-model>` route and every
explicit zero-cost `openrouter/<model>` route listed in `dsh_models`.

All OpenCode and OpenRouter reviews use transient agents inside one headless DSH
process. Recipe Machine sends prompts over an owner-only Unix socket. DSH routes
each agent directly through its configured `opencode` or `openrouter` adapter,
keeps one agent only for the initial decision and optional schema-repair turn,
then disposes it. No Web UI, OpenCode server, or per-review client process runs.
The controller remains the single admission boundary, so queue depth never
materializes thousands of idle agents. Model turns run concurrently only while
the controller grants review leases. Every turn uses the configured deadline,
and retiring a lease cancels the live DSH agent immediately.

The OpenRouter adapter resolves its API key from OpenCode's existing
`auth.json` on every request. Recipe Machine neither copies the key into DSH
configuration nor creates a second authorization flow. OpenCode Zen remains a
keyless route. The machine uses each provider until that provider is
unavailable. It does not wait for consensus. When a route fails, the machine
records its reported reset duration and does not dispatch that route again
before the reset. A diagnostic without a reset duration uses the configured
bounded cooldown, then measures the route again. If every route is unavailable,
the reviewer retains the failure while cursor discovery continues.

## Classifier access

Every classifier can read the Recipe repository and call only two GitHub tools:

- `search_issues` searches the complete Recipe issue history and returns issue
  numbers, titles, states, and URLs.
- `read_issue` returns one complete issue, including every comment.

The classifiers search issues on demand and read every plausible match in full.
The machine does not preload, cache, summarize, or truncate the issue tree.
DSH exposes the issue reader to classifiers through its project-scoped MCP
configuration.

The classifier tool surfaces contain only repository readers and the two issue
readers. A classifier cannot edit files, run shell commands, delegate work,
create branches, commit, push, publish issues, or open pull requests.

Each provider keeps its real conversation or session identifier. If the final
assistant response does not match `decision.schema.json`, the machine asks that
same conversation to return a corrected JSON object. Authentication failures,
usage-limit failures, and unavailable models fall through without a repair
turn.

## Publication

The Rust machine owns the only GitHub write boundary. It serializes publication
and scans the configured issue history before accepting a `new` decision. If an
existing issue has the same title, the machine comments on the earliest matching
issue. Otherwise, it creates an issue with the `bug` label. For a `comment`
decision, it comments on the selected issue. Every published issue or comment
records:

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
   ├─ openrouter/north-mini-code       cursor 570  time 12.0031 s
   └─ opencode/mimo-v2.5-free          cursor 536  time 51.0867 s
└─ resolve
   ├─ claude/fable-5-high              issue #171  time 51.1012 s
   └─ claude/fable-5-high              issue #172  time 49.3001 s
```

The terminal groups a provider under one node only while multiple models from
that provider are active. A provider with one active model stays on one line.
Long model identities truncate before the cursor and elapsed time, so live
timing remains visible at the current pane width.

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

## Control the machine

Restart the service after changing the machine source or configuration, then
attach to the persistent tmux session:

```text
systemctl --user restart recipe-issue-machine.service
tmux attach -t recipe-issue-machine
```

Configuration is in `machine.toml`. The Recipe repository uses a dedicated
worktree on the `machine` branch, so traversal state cannot alter Nate's active
checkout. `machine.toml` stores the resumable cursor. GitHub issues and comments
remain the authoritative deduplication state. `queue.ogdl` is the durable
pending-review state. A packet remains there until its decision is published,
and a restart resumes the oldest queued packet first.
