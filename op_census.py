#!/usr/bin/env python3
# Census of gpu-core public function signatures: inputs, outputs, frequencies.
import re, os, sys, json
from collections import Counter

SRC = os.path.join(os.path.dirname(os.path.abspath(__file__)), "gpu-core", "src")
PLUMBING = {"hip", "memory", "tiered", "waterfall", "hw", "callspy", "lib"}

def strip_comments(text):
      # remove // comments (naive but fine for this codebase; no /* */ blocks in sigs)
      out = []
      for line in text.split("\n"):
            # avoid cutting inside string literals containing // (rare in sigs)
            idx = line.find("//")
            if idx >= 0 and '"' not in line[:idx]:
                  line = line[:idx]
            out.append(line)
      return "\n".join(out)

def norm_type(t):
      t = re.sub(r"'[a-z_]+\s*", "", t)          # lifetimes
      t = re.sub(r"\s+", " ", t).strip()
      t = t.replace("& ", "&").replace("&mut ", "&mut~")  # protect mut
      t = re.sub(r"\s*([<>,()])\s*", r"\1", t)
      t = t.replace("&mut~", "&mut ")
      t = re.sub(r"Result<(.+?),\s*HipError>", r"Result<\1>", t)
      t = re.sub(r"Result<(.+?),\s*anyhow::Error>", r"Result<\1>", t)
      t = t.replace("crate::memory::GpuBuffer", "GpuBuffer")
      return t

def split_top(s, sep=","):
      parts, depth, cur, prev = [], 0, "", ""
      for ch in s:
            if ch in "<([" : depth += 1
            if ch in ")]" : depth -= 1
            if ch == ">" and prev != "-": depth -= 1   # '->' is not a closing angle
            if ch == sep and depth == 0:
                  parts.append(cur); cur = ""
            else:
                  cur += ch
            prev = ch
      if cur.strip(): parts.append(cur)
      return [p.strip() for p in parts if p.strip()]

rows = []
counts = {"pub fn": 0, "pub unsafe fn": 0, "pub(crate) fn": 0}
for fname in sorted(os.listdir(SRC)):
      if not fname.endswith(".rs"): continue
      mod = fname[:-3]
      raw = open(os.path.join(SRC, fname)).read()
      # `// not-an-op: <reason>` on the line above a pub fn excludes it from the op
      # set (drivers/plumbing/plan-helpers). Audited: --check prints every one.
      annotated = {}
      raw_lines = raw.split("\n")
      for ln, line in enumerate(raw_lines):
            m = re.search(r"//\s*not-an-op:\s*(.+)", line)
            if m:
                  for la in range(ln + 1, min(ln + 3, len(raw_lines))):
                        fm = re.search(r"\bpub(\(crate\))?\s+(unsafe\s+)?fn\s+(\w+)", raw_lines[la])
                        if fm:
                              annotated[fm.group(3)] = m.group(1).strip()
                              break
      text = strip_comments(raw)

      # map char pos -> inside extern block? track impl context by brace depth
      # build events: (pos, kind, name)
      ctx_impl = []   # stack entries: (depth_at_open, impl_type or None, is_extern)
      depth = 0
      i = 0
      # Pre-find headers: impl blocks and extern blocks with their opening brace pos
      headers = []
      for m in re.finditer(r"^\s*impl(?:<[^>]*>)?\s+(?:[\w:]+\s+for\s+)?([\w:]+(?:<[^>{]*>)?)", text, re.M):
            headers.append((m.start(), "impl", m.group(1)))
      for m in re.finditer(r'extern\s*"C"\s*\{', text):
            headers.append((m.start(), "extern", None))
      headers.sort()

      # brace-depth walk assigning context to each pub fn
      fn_pat = re.compile(r"\bpub(\(crate\))?\s+(unsafe\s+)?fn\s+(\w+)")
      # compute depth at every header/fn position via single pass
      events = sorted([(m.start(), "fn", m) for m in fn_pat.finditer(text)] + headers)
      pos_depth = {}
      d = 0; ei = 0
      for p, ch in enumerate(text):
            while ei < len(events) and events[ei][0] == p:
                  pos_depth[p] = d; ei += 1
            if ch == "{": d += 1
            elif ch == "}": d -= 1
      # active contexts: list of (open_depth, kind, tag) — a header at depth d owns fns at depth d+1
      # simpler: for each fn, the nearest preceding header whose brace region contains it.
      # approximate: nearest preceding header at pos_depth[fn]-1 with no intervening depth drop below it.
      for p, kind, m in events:
            if kind != "fn": continue
            crate_vis, uns, name = m.group(1), m.group(2), m.group(3)
            key = "pub(crate) fn" if crate_vis else ("pub unsafe fn" if uns else "pub fn")
            counts[key] = counts.get(key, 0) + 1
            # capture signature: from match end, balanced parens for args
            j = text.find("(", m.end())
            if j == -1: continue
            dep, k = 0, j
            while k < len(text):
                  if text[k] == "(": dep += 1
                  elif text[k] == ")":
                        dep -= 1
                        if dep == 0: break
                  k += 1
            args_raw = text[j+1:k]
            # return type: up to '{' or ';' or 'where'
            rest = text[k+1:k+400]
            body_end = len(rest)
            for stop in ["{", ";"]:
                  q = rest.find(stop)
                  if q != -1: body_end = min(body_end, q)
            wq = rest.find("where")
            if wq != -1: body_end = min(body_end, wq)
            ret_seg = rest[:body_end]
            is_decl = rest[body_end:body_end+1] == ";"
            ret = "()"
            if "->" in ret_seg:
                  ret = norm_type(ret_seg.split("->", 1)[1])
            # context: nearest preceding header at depth-1
            fd = pos_depth.get(p, 0)
            ctx_kind, ctx_tag = None, None
            for hp, hk, ht in reversed(headers):
                  if hp < p and pos_depth.get(hp, 0) == fd - 1:
                        ctx_kind, ctx_tag = hk, ht
                        break
                  if hp < p and pos_depth.get(hp, 0) < fd - 1:
                        break
            # parse args
            ins = []
            recv = None
            for a in split_top(args_raw):
                  if a in ("&self", "&mut self", "self"):
                        recv = a.replace("self", ctx_tag or "Self")
                        continue
                  if ":" in a:
                        ins.append(norm_type(a.split(":", 1)[1]))
                  else:
                        ins.append(norm_type(a))
            kind2 = "ffi" if (ctx_kind == "extern" or is_decl) else ("plumbing" if mod in PLUMBING else "op")
            line_no = text.count("\n", 0, p) + 1
            rows.append(dict(module=mod, kind=kind2, name=name, line=line_no, recv=recv,
                             inputs=ins, output=ret, not_an_op=annotated.get(name)))

print(json.dumps(counts), file=sys.stderr)
print(f"total parsed: {len(rows)}", file=sys.stderr)

# ── dump ──
dump_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "op_census_dump.tsv")
with open(dump_path, "w") as f:
      f.write("kind\tmodule\tfn\tline\treceiver\tinputs\toutput\n")
      for r in sorted(rows, key=lambda r: (r["kind"], r["module"], r["name"])):
            f.write(f"{r['kind']}\t{r['module']}\t{r['name']}\t{r['line']}\t{r['recv'] or ''}\t{', '.join(r['inputs'])}\t{r['output']}\n")
print(f"dump: {dump_path}", file=sys.stderr)

# ── --check: enforce the schedulable-op type constraint ──────────────────────
# fn op(in: &GpuBuffer ..., dim: usize ..., out: &GpuBuffer ...) -> Result<()>
# inputs only &GpuBuffer/usize; return exactly Result<()>; no methods.
ALLOW = {
      "gpu_shutdown",        # device lifecycle, not a schedulable op
}
def is_plan_helper(r):
      # plan-time size query: no device work — only usize in, usize out
      return all(t == "usize" for t in r["inputs"]) and r["output"] == "usize" \
            and r["name"].endswith("_workspace_bytes")

if "--check" in sys.argv:
      excluded = [r for r in rows if r["kind"] == "op" and r["not_an_op"]]
      viols = []
      for r in rows:
            if r["kind"] != "op" or r["name"] in ALLOW or is_plan_helper(r) or r["not_an_op"]:
                  continue
            why = []
            if r["recv"]:
                  why.append(f"method receiver {r['recv']}")
            for t in r["inputs"]:
                  if t not in ("&GpuBuffer", "usize"):
                        why.append(f"input {t}")
            if r["output"] != "Result<()>":
                  why.append(f"returns {r['output']}")
            if why:
                  viols.append((r, why))
      by_clause = Counter()
      for r, why in viols:
            for w in why:
                  by_clause[w.split(" ")[0] + " " + (w.split(" ")[1] if w.startswith(("input", "returns")) else "")] += 1
      for r, why in sorted(viols, key=lambda v: (v[0]["module"], v[0]["line"])):
            print(f"VIOLATION gpu-core/src/{r['module']}.rs:{r['line']} {r['name']}: {'; '.join(why)}")
      n_ops = sum(1 for r in rows if r["kind"] == "op" and r["name"] not in ALLOW
                  and not is_plan_helper(r) and not r["not_an_op"])
      print(f"\n--check: {n_ops} ops, {n_ops - len(viols)} conforming, {len(viols)} violating")
      for clause, n in by_clause.most_common():
            print(f"  {n:4d}  {clause}")
      if excluded:
            print(f"\nnot-an-op exclusions ({len(excluded)}) — audit this list:")
            for r in sorted(excluded, key=lambda r: (r["module"], r["line"])):
                  print(f"  gpu-core/src/{r['module']}.rs:{r['line']} {r['name']}: {r['not_an_op']}")
      sys.exit(1 if viols else 0)

def bars(counter, total, top=None, width=36):
      items = counter.most_common(top)
      mx = items[0][1] if items else 1
      out = []
      for t, c in items:
            b = "█" * max(1, round(c / mx * width))
            out.append(f"{c:5d}  {c/total*100:5.1f}%  {b}  {t}")
      return "\n".join(out)

ops = [r for r in rows if r["kind"] == "op"]
print(f"\n=== SCOPE: {len(ops)} compute ops | {sum(1 for r in rows if r['kind']=='ffi')} ffi launchers | {sum(1 for r in rows if r['kind']=='plumbing')} plumbing fns ===")

# output freq
out_c = Counter(r["output"] for r in ops)
print(f"\n=== OUTPUT TYPES ({len(ops)} ops) ===")
print(bars(out_c, len(ops)))

# individual input param freq
in_params = Counter()
for r in ops:
      for t in r["inputs"]:
            in_params[t] += 1
      if r["recv"]: in_params["&" + r["recv"].lstrip("&").replace("mut ", "")] += 0  # receivers rare in op modules
tot_params = sum(in_params.values())
print(f"\n=== INPUT PARAMETER TYPES ({tot_params} params across {len(ops)} ops) ===")
print(bars(in_params, tot_params, top=25))

# input multiset freq
def sig_of(r):
      c = Counter(r["inputs"])
      return ", ".join(f"{t}×{n}" if n > 1 else t for t, n in sorted(c.items(), key=lambda x: (-x[1], x[0])))
sig_c = Counter(sig_of(r) for r in ops)
print(f"\n=== INPUT SIGNATURE SETS (top 30 of {len(sig_c)} distinct) ===")
print(bars(sig_c, len(ops), top=30))

# full-shape freq: inputs -> output
shape_c = Counter(f"({sig_of(r)}) -> {r['output']}" for r in ops)
print(f"\n=== FULL SHAPES inputs->output (top 20 of {len(shape_c)} distinct) ===")
print(bars(shape_c, len(ops), top=20))
