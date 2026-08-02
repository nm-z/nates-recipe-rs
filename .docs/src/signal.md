# `src/signal.rs`: process-scoped SIGINT request transport

```toml
[module]
path = "src/signal.rs"
kind = "private-process-signal-boundary"
intent = "Convert SIGINT into one process-local atomic stop request, and restore the prior SIGINT action when the owning operation exits."
purpose = "Give dense training a safe-boundary stop source and let the source-runner parent forward one request to its compiled child."
state = "one AtomicBool plus one installation Mutex per process"
side_effects = ["install or restore the process SIGINT disposition", "send SIGINT to one validated child pid"]

[boundary]
inputs = ["operating-system SIGINT", "validated child process id as u32", "one active guard per process"]
outputs = ["AtomicBool request flag", "io::Result<SigintGuard>", "io::Result<()> from send_sigint"]
public_surface = "crate-private; the module is not re-exported by src/lib.rs"
signal_handler_work = "one AtomicBool Release store only"
training_stop_point = "after a complete loop iteration, before another iteration begins"
artifact_behavior = "the ordinary post-exit save declarations run; no save is initiated by this module"

[non_goals]
counts = "no signal count, timestamp, sender identity, or queue"
payload = "no signal-dependent data is read or written by the handler"
termination = "no forced process exit and no direct native-resource teardown"
recovery = "no retry, alternate handler, or poison recovery"
```

`src/lib.rs` includes this file as the private `signal` module. The only
production call sites are `src/training.rs:889-895`, which installs a guard for
a dense `Train::run`, and `src/cli.rs:439-489`, which installs a separate guard
around a compiled child and calls `send_sigint` once. `src/main.rs` only calls
`recipe::cli::main()`. No public API lets a caller install this handler or read
the flag directly.

## Structure

The module has two process-global objects, one handler, one RAII guard, and one
child-signal helper:

| source region | item | role and lifetime |
| --- | --- | --- |
| `src/signal.rs:12` | `SIGINT_REQUESTED: AtomicBool` | Shared request state. It starts `false`, is set to `true` by `record_sigint`, is cleared after the installation lock is acquired and before `sigaction`, and remains at its last value after guard drop until the next installation. |
| `src/signal.rs:13` | `SIGINT_INSTALLATION: Mutex<()>` | Process-wide installation serialization. The lock guard is stored inside `SigintGuard`, so it remains held from `install` success through `Drop`. |
| `src/signal.rs:15-19` | `record_sigint` | `extern "C"` signal handler. It ignores the signal number and performs only `SIGINT_REQUESTED.store(true, Ordering::Release)`. |
| `src/signal.rs:22-25` | `SigintGuard` | Private RAII owner of the previous `libc::sigaction` and the installation mutex guard. |
| `src/signal.rs:36-67` | `SigintGuard` methods | Install the action, expose an acquire read, and expose the static request flag to the controlled executor. |
| `src/signal.rs:69-77` | `Drop for SigintGuard` | Restores the saved action. The `sigaction` return value is ignored. The mutex guard is released after the custom `drop` body returns. |
| `src/signal.rs:79-89` | `send_sigint` | Converts a `u32` child id to `libc::pid_t`, calls `libc::kill(pid, SIGINT)`, and returns the OS result. It does not wait or retry. |

The file opts out of the workspace unsafe-code lint at line 1 because the
handler and the `sigaction` and `kill` calls cross the libc boundary. The safety
comments describe the assumptions used by the implementation: the C ABI
handler performs one atomic store, both `sigaction` values are initialized
before use, and `kill` does not dereference process memory.

## Installation and restoration

`SigintGuard::install()` is the complete registration transaction
(`src/signal.rs:37-62`):

1. Lock `SIGINT_INSTALLATION`. A poisoned mutex becomes
   `io::Error::other("Recipe SIGINT installation lock is poisoned")` and no
   signal action is changed.
2. Clear `SIGINT_REQUESTED` with `Ordering::Release`. This starts a new
   operation with no request carried over from an earlier guard.
3. Zero a `libc::sigaction`, set `sa_sigaction` to `record_sigint`, set
   `sa_flags` to zero, and clear its mask with `sigemptyset`.
4. If `sigemptyset` fails, return `io::Error::last_os_error()`. The prior
   process disposition remains in effect because `sigaction` has not yet been
   called; the lock is released while returning the error.
5. Call `sigaction(SIGINT, &action, &mut previous)`. A failure returns the OS
   error and leaves the prior disposition in effect. A success stores the exact
   returned `previous` action in the guard and returns it while retaining the
   mutex lock.

The installed action has no custom flags and an empty explicit mask. The
handler does not chain the previous action, print, allocate, lock, format, or
perform a system call. It only records that a request occurred. The source
does not make a second registration attempt if any step fails.

`Drop` calls `sigaction(SIGINT, &_previous, null_mut())` with the lock still
held (`src/signal.rs:69-76`). This restores the action that was observed by the
successful installation call. Restoration errors are discarded. Once the
`Drop` body ends, `_installation` is dropped and another installation may
proceed. The atomic flag is not cleared by `Drop`; the next `install` clears it
before configuring its new action.

Because the previous action is saved only once and the lock is retained for the
whole guard lifetime, a second `install` in the same process waits rather than
creating overlapping guards. A panic while holding the mutex can poison it;
subsequent calls fail at step 1 because the implementation does not recover a
poisoned lock. The source also does not detect an external party changing the
SIGINT action while a guard is live, so normal restoration can replace such a
change with the saved action.

## Inputs and outputs

### Operating-system SIGINT

The operating system invokes `record_sigint(_signal: libc::c_int)` while the
Recipe action is installed. The signal number is intentionally unused. The
handler's only observable output is the `true` value in
`SIGINT_REQUESTED`. Multiple signals are idempotent at this boundary: there is
no count or ordering information, and the request remains `true` until a later
installation clears it.

The Release store pairs with the Acquire loads in `requested()` and in the
training control (`training/src/execute.rs:113-116`). The handler does not
touch model weights, executor state, files, stdout, or native handles.

### `requested()`

`SigintGuard::requested()` (`src/signal.rs:64`) performs one
`SIGINT_REQUESTED.load(Ordering::Acquire)`. It is used by the CLI parent polling
loop (`src/cli.rs:474-489`) to decide whether to forward a request. It is not a
blocking wait and it does not clear the flag.

### `request_flag()`

`SigintGuard::request_flag()` (`src/signal.rs:66`) returns
`&'static AtomicBool` for the same process-global flag. The root training code
passes this pointer immediately to `execute_current_training`, then to
`execute_current_training_native`, and finally to
`TrainingExecutionControl::graceful_stop`. The pointer is a capability to read
the request at executor boundaries, not a second state object and not a public
signal API.

### `send_sigint(process_id)`

`send_sigint` accepts the `u32` returned by `std::process::Child::id()` in the
CLI caller. It first performs `libc::pid_t::try_from`; a value that does not
fit returns `io::ErrorKind::InvalidInput` with
`process id does not fit pid_t`. A successful `libc::kill(pid, libc::SIGINT)`
returns `Ok(())`; any nonzero return becomes `io::Error::last_os_error()`.
The helper does not inspect process state, wait for delivery, retry, or send a
second signal. The caller decides how to treat `ESRCH` and other errors.

## Dense training flow

The request flag is connected to training only on the dense native path. The
full path is:

```text
recipe.train().run()
  -> Train::try_run_with
  -> compile_training_package
  -> SigintGuard::install
  -> execute_current_training(..., guard.request_flag())
  -> execute_current_training_native(..., stop_requested)
  -> TrainingExecutionControl::graceful_stop(stop_requested)
  -> prepare_and_execute_local_training_controlled
  -> RunningRun::poll_with_progress_or_stop
```

`Train::try_run_with` first consumes the data/model sequence for the ordinary
`.run()` form (`src/training.rs:858-867`). It then selects a non-native branch
for Bayesian or KNN preparation, or the dense branch at lines 888-916:

| branch | handler installation | stop/report behavior |
| --- | --- | --- |
| Bayesian model | none | Compile the semantic model and optionally save it. `TrainingReport::bayes` sets `gracefully_stopped` to `false`. |
| KNN model | none | Prepare the immutable reference model and optionally save it. `TrainingReport::knn` sets `gracefully_stopped` to `false`. |
| dense model | after `compile_training_package` succeeds | Install the guard, execute through the controlled native entrypoint, construct a completed checkpoint, then save declared artifacts. |

Compilation, data preparation, resume decoding, and native-kernel resume
validation happen before the dense guard is installed
(`src/training.rs:581-590`, `592-758`, and `761-846`). A failure in those
stages does not go through `src/signal.rs`.

After installation, `execute_current_training` keeps the guard alive while it
reports validation availability, optionally runs the live metric presenter,
executes the native lifecycle, drains final metrics, and joins the presenter
(`src/training.rs:920-957`). Any execution or presentation error propagates
before a report is built and therefore before a save call.

`execute_current_training_native` prepares the measured native scope, validates
any supplied native resume identity, creates the backend and pre-loop
realizer, and calls `prepare_and_execute_local_training_controlled` with
`TrainingExecutionControl::graceful_stop(stop_requested)`
(`src/training.rs:1278-1337`). The signal handler itself never enters that
closure and never observes backend state.

## Safe-boundary observation and state effects

`TrainingExecutionControl` is a read-only wrapper around an optional
`&AtomicBool` (`training/src/execute.rs:90-119`). Its `stop_requested()` method
performs an Acquire load. The controlled entrypoint rejects an unbounded loop
when the wrapper has no source (`training/src/execute.rs:121-128`); the root
dense terminal always supplies the signal flag.

The controlled executor prepares the immutable program, validates loop
boundaries, packs one init image per device, constructs `PreparedRun`, admits
the images, and starts the loop before it reads the flag
(`training/src/execute.rs:2176-2235`). It then repeats this sequence
(`training/src/execute.rs:2236-2259`):

1. `RunningRun::poll_with_progress_or_stop` executes one bounded scheduler and
   backend poll pass for the active iteration.
2. The pass completes only after every active loop task, transfer, metric, and
   fault readback has reached its terminal state. A pending calculation or
   transfer is never interrupted by the flag.
3. Training drains the latest user metric values. A bounded live observer may
   drop a notification when its channel is full or disconnected, but observer
   state cannot backpressure execution.
4. When the phase is complete, the executor records
   `LogicalEvent::LoopIterationCompleted`, then invokes the stop closure.
5. A `true` load suppresses the next iteration, records
   `LogicalEvent::LoopStopAccepted { after_iteration }`, records
   `LogicalEvent::LoopCompleted`, and returns `LoopStatus::Complete`.
6. A `false` load starts the next configured iteration when one exists. At a
   finite endpoint there is no next iteration, so `LoopCompleted` is recorded
   normally. If the flag is true at that endpoint, `LoopStopAccepted` is also
   recorded.

The executor therefore preserves all calculations, transfers, recurrent
parameters, and optimizer state from the last completed iteration. A request
that arrives while a phase is active is observed only after that phase reaches
completion. A request that races with the boundary load may permit the next
iteration to start; the source defines no stronger cancellation point.

After a stop or a finite endpoint, `into_exited_loop` is legal only when the
loop phase is complete and no failure was recorded. `ExitedLoop::exit` runs the
ordinary exit transfers, collects external output images, releases every arena,
destroys native resources, and records `LogicalEvent::Exited`
(`executor/src/executor.rs:1293-1371`). `CompletedTrainingExecution` is returned
only after this teardown, so a graceful request never returns a live native
session.

The public report derives its stop state from the journal rather than reading
the atomic (`src/training.rs:223-239`): `TrainingReport::dense` sets
`gracefully_stopped` exactly when a `LoopStopAccepted` event exists. The public
`TrainingReport::gracefully_stopped()` method documents this as a host stop
accepted after a complete epoch (`src/training.rs:339-349`).

The executor reserves the optional stop event in its fixed lifecycle journal
capacity (`executor/src/executor.rs:323-333`). `RunJournal` compacts repeated
loop task, metric, fault, and iteration events after the first iteration, but
`LoopStopAccepted` is not in that repeated-event set
(`executor/src/executor.rs:2743-2757`), so the stop evidence remains ordered and
available for the report even when an unbounded run has observed many epochs.

## Save and resume behavior after a request

The signal module only transports a request. It does not choose an artifact,
write a file, or call checkpoint code. Once the controlled executor returns,
the dense branch constructs `TrainingReport::dense`, then executes the ordinary
save sequence (`src/training.rs:897-916`):

1. If a model destination was declared, `report.save_model(destination)` writes
   the completed semantic `.ogdl` checkpoint. Its output images represent the
   final parameter and optimizer state retained by the completed run.
2. If a native-kernel destination was declared, its extension selects `Cubin` or
   `Hsaco`, and `report.save_native_kernel` writes the exact realized image.
3. If no destination was declared, no user-owned artifact is written.

The sequence is independent of whether `gracefully_stopped` is true. A stop
accepted at a complete epoch therefore follows the same exit and save path as a
finite endpoint, and a model save remains independent of `.resume(...)`. A
model-save failure returns before the optional kernel save; a kernel-save
failure returns after any successful model save. In either case Rust drops the
guard while unwinding, restoring the prior SIGINT action. An execution,
presenter, report-construction, or teardown error similarly skips both save
calls and still drops the guard.

The guard remains in scope through report construction and both save calls. A
SIGINT received during these post-exit operations can set the atomic, but there
is no subsequent loop boundary in this call to consume it. The source does not
restart a save or turn that late request into another stop event.

## CLI parent and child flow

`recipe run FILE.rs` has a second, process-level use of the module. After source
lowering and compilation, `run_compiled_source_live` installs a guard in the
parent `recipe` process (`src/cli.rs:433-447`), starts the compiled binary, and
forwards output from two reader threads. Its status loop is:

```text
parent SIGINT
  -> parent record_sigint
  -> parent requested() Acquire load
  -> forwarded_interrupt = true
  -> send_sigint(child.id()) once
  -> child process receives SIGINT
  -> dense child Train::run installs its own guard and records its own flag
```

The parent checks the flag every 10 milliseconds (`src/cli.rs:474-499`). The
local `forwarded_interrupt` boolean suppresses all later forwards. A successful
send continues polling. `ESRCH` is accepted because the child has already
exited. Any other send error kills and waits for the child and returns a
forwarding error. A `try_wait` error also kills and waits. The parent does not
apply a timeout or escalate a second Ctrl+C.

The parent and child are separate processes, so each has an independent static
atomic, mutex, and SIGINT disposition. The parent never passes its atomic
pointer into the child. The child must reach the dense `SigintGuard::install`
call before its process-local request can become a graceful executor stop. The
source runner's parent guard still catches and forwards a signal during child
compilation or other child phases, but those phases have no child-side signal
registration in this module. Bayesian and KNN training, inference, probing,
and conversion likewise have no dense training guard. Their child behavior on a
forwarded signal is whatever SIGINT action is active in that child process.

After child exit, the parent joins both output forwarders. Returning from
`run_compiled_source_live` drops the parent guard and restores the previous
action. `run_source` then removes the temporary compiled binary before
interpreting the child status (`src/cli.rs:344-360`). A child that did not
handle SIGINT gracefully is reported as a non-success status such as
`terminated by signal 2`; a graceful dense child can return success after its
exit and declared saves.

## Concurrency and memory ordering

The handler and executor deliberately communicate through one atomic bit:

```text
record_sigint:          AtomicBool.store(true, Release)
SigintGuard::requested: AtomicBool.load(Acquire)   [CLI parent]
TrainingExecutionControl::stop_requested:
                         AtomicBool.load(Acquire)   [executor boundary]
```

The handler performs no operation that requires a lock or allocator and does
not call into Recipe. Native worker threads can continue their current backend
work; the host polling thread is the only code that turns the observed bit into
a lifecycle transition. The bit is process-scoped, not run-scoped, so a new
guard must clear it before beginning a new operation.

`SIGINT_INSTALLATION` serializes guard installation and restoration within one
process. It cannot coordinate parent and child processes and it does not
serialize unrelated external `sigaction` calls. Standard SIGINT delivery and
the Boolean representation mean that several signals collapse to one request.
There is no channel, queue, counter, or acknowledgment from the child back to
the parent.

Concurrent dense callers can compile or prepare independently before they reach
their guard call, but only one caller can own an installed Recipe action at a
time. A caller that reaches `install` while another guard is live waits for that
guard to restore the previous action. The second caller then clears the shared
bit before its own execution, so the bit does not identify a particular run.

The CLI output-forwarder threads do not access the signal guard. They only read
and flush child output. The parent polling thread owns the guard's
`requested()` calls, while the dense training host thread owns the executor
callback. This keeps all signal state observation explicit and bounded.

## Failure paths and limits

| operation | failure | direct consequence |
| --- | --- | --- |
| mutex lock in `install` | poisoned `SIGINT_INSTALLATION` | `io::Error` with the fixed poison message; no new action is installed |
| `sigemptyset` | nonzero libc result | return `last_os_error`; prior action remains; no guard is returned |
| `sigaction` installation | nonzero libc result | return `last_os_error`; prior action remains; no guard is returned |
| `sigaction` restoration in `Drop` | nonzero libc result | ignored; the lock is still released after `Drop`, so a stale installed action is possible if the OS rejects restoration |
| `send_sigint` pid conversion | `u32` does not fit `pid_t` | `InvalidInput`; no `kill` call |
| `send_sigint` kill | OS rejects the signal | return the OS error; the CLI treats `ESRCH` as already exited and all other errors as fatal after best-effort child kill/wait |
| unbounded lower executor | no `TrainingExecutionControl` source | `UnboundedTrainingRequiresStopControl`; the signal module is not implicitly installed by lower entrypoints |
| dense guard installation | any `install` error | `TrainingError::Runtime` at stage `install graceful SIGINT handler`; no native execution or save |
| dense execution or exit | preparation, backend, watchdog, journal, metric, output, or teardown error | `TrainingError` propagates; no completed report and no declared save calls |
| report or artifact write | checkpoint/report or model/kernel write error | `TrainingError::Checkpoint` (or the originating typed error); later saves are skipped; guard restoration still runs on return/unwind |
| CLI child send/wait/output | forwarding, wait, pipe, or forwarder join error | child is killed/waited where the caller has a cleanup branch, output threads are joined, and the CLI returns a human-readable error |

There is no signal-specific error variant and no signal-specific log record.
The only successful stop evidence is `LogicalEvent::LoopStopAccepted` in the
completed dense run journal and the derived public
`TrainingReport::gracefully_stopped()` value. A set flag without a completed
executor lifecycle is not a successful graceful stop.

## Boundaries and invariants

* The handler is process-scoped and private. It is not an application event
  bus, a reusable cancellation token, or a public signal customization point.
* The handler records intent only. The executor owns the valid state transition
  and checks the flag only at a completed loop-iteration boundary.
* No handler path touches GPU memory, host model state, files, output streams,
  checkpoint manifests, or native resources.
* Dense training gets one guard after compilation and before native execution.
  KNN, Bayesian preparation, inference, probing, conversion, and lower-level
  uninterrupted execution do not acquire this module's handler.
* A graceful stop runs the ordinary `init -> loop -> exit` lifecycle already in
  progress. It does not create a partial epoch, a synthetic epoch count, a
  retry, or an intermediate checkpoint.
* The final save decision remains the caller's independent `.save(...)`
  declaration. Omitting `.save(...)` exports nothing even when a stop is
  accepted.
* Parent forwarding is one-shot. The parent remains alive while the child
  quiesces; a second Ctrl+C does not force a different path.

## Source evidence map

| behavior | evidence |
| --- | --- |
| global flag, mutex, handler, guard, restoration, child kill helper | `src/signal.rs:1-89` |
| private module inclusion | `src/lib.rs:1-7` |
| dense guard installation, request handoff, report and saves | `src/training.rs:848-957` |
| compile/resume work before installation | `src/training.rs:581-846` |
| native controlled handoff | `src/training.rs:1278-1337`; `training/src/execute.rs:90-128`, `2176-2294` |
| safe boundary and stop journal events | `executor/src/executor.rs:1155-1249` |
| exit transfer and ordered teardown | `executor/src/executor.rs:1293-1371`, `1489-1553` |
| CLI parent installation and one-shot forwarding | `src/cli.rs:272-360`, `433-508` |
| source frontend call to hidden training entrypoint | `src/source_frontend.rs:493-507`; `src/facade.rs:848-867` |
| contract-level unbounded and graceful-stop behavior | `system-contract.md:457-474` |
| process acceptance path | `acceptance/src/main.rs:842-958` |
