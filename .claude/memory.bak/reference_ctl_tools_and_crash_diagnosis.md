---
name: reference_ctl_tools_and_crash_diagnosis
description: Diagnose GPU aborts/core dumps with coredumpctl (userspace backtrace) + journalctl -k (amdgpu page-fault detail = fault-vs-OOM); full inventory of *ctl tools installed on engi
metadata: 
  node_type: memory
  type: reference
  originSessionId: 9d550ed6-652b-46a9-a0d0-5432a864d444
---

## GPU crash diagnosis workflow (use this, don't theorize)

When a binary dies with "Aborted (core dumped)" / SIGABRT / SIGSEGV, get GROUND TRUTH from these before hypothesizing:

- **`coredumpctl list`** — recent dumps (PID, signal, exe, present?).
- **`coredumpctl info <PID>`** — userspace backtrace across all threads. On this project SIGABRTs the top of the aborting thread is `libhsa-runtime64 → abort`, and another thread shows the real call chain (e.g. `main → Train::run → recipe_infer::forward::zscore_apply → GpuBuffer::alloc_bytes → hipMallocAsync`). Release builds have locals optimized out, so `coredumpctl gdb` → `info locals` gives "No symbol table". Do NOT rebuild over the binary before inspecting — it invalidates the core's symbol match.
- **`journalctl -k`** (kernel ring buffer) — THE decisive source for GPU faults. `amdgpu … [gfxhub] page fault … client 10 (TCP) … PERMISSION_FAULTS RW:0x1` + `sq_intr: error type 2 priv 1` = a **GPU memory-access/page fault** (shader wrote an unmapped page), which HSA reports at the NEXT HIP call — NOT out-of-memory. This is how "it's OOM" was disproven; the abort backtrace's `alloc_bytes` frame is where the async fault surfaced, not where it originated. See [[project_gpu_async_free_fault]] and [[project_hip_oom_asserts]].
  - Scope it: `journalctl -k --since "10 min ago"`, `journalctl -k -b` (this boot), grep `amdgpu|page fault|sq_intr|ABRT`.
  - Userspace unit logs: `journalctl _COMM=cookbook` or `journalctl --user -b`.

**Never touch processes** ([[feedback_never_touch_processes]]): no kill/pkill/rocm-smi. coredumpctl/journalctl are read-only forensics on already-dead processes — always allowed.

## Full *ctl inventory on engi (from `compgen -c | rg ctl`)

Crash/system: `coredumpctl`, `flatpak-coredumpctl`, `journalctl`, `systemctl`, `busctl`, `oomctl` (systemd-oomd OOM state — relevant to memory pressure), `auditctl`, `varlinkctl`, `importctl`, `portablectl`, `machinectl`, `userdbctl`, `homectl`, `loginctl`, `hostnamectl`, `localectl`, `timedatectl`, `bootctl`, `sysctl`.
Storage/mem: `zramctl`, `numactl`, `smartctl`, `udisksctl`, `storagectl`, `daxctl`, `ndctl`, `wdctl`, `balooctl6` (KDE file-indexer — spams the journal with portal errors).
Net: `networkctl`, `resolvectl`, `netctl`, `netctl-auto`, `iwctl`, `bluetoothctl`, `boltctl` (thunderbolt), `rds-ctl`, `teamdctl`, `btpclientctl`.
Audio/media/input: `pactl`, `wpctl`, `sndioctl`, `rtkitctl`, `v4l2-ctl`, `ivtv-ctl`, `cx18-ctl`, `ir-ctl`, `cec-ctl`, `media-ctl`, `ratbagctl` (mouse).
Desktop/GPU/misc: `hyprctl`, `gamescopectl`, `panelctl`, `plugctl`, `keyctl`, `updatectl`, `cupsctl`, `pkgctl`, `idevicedevmodectl`.

Prefer the right `*ctl` over guessing/parsing files: memory pressure → `oomctl` + `journalctl -k`; disk health → `smartctl`; services → `systemctl`; swap → `zramctl`.
