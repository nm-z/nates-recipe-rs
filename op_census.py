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
      inplace = {}
      raw_lines = raw.split("\n")
      for ln, line in enumerate(raw_lines):
            ip = re.search(r"//\s*in-place:\s*(.+)", line)
            if ip:
                  for la in range(ln + 1, min(ln + 30, len(raw_lines))):
                        fm = re.search(r"\bpub(\(crate\))?\s+(unsafe\s+)?fn\s+(\w+)", raw_lines[la])
                        if fm:
                              inplace[fm.group(3)] = ip.group(1).strip()
                              break
                        t = raw_lines[la].strip()
                        if t and not t.startswith(("//", "#[", "#!")):
                              break
            m = re.search(r"//\s*not-an-op:\s*(.+)", line)
            if not m:
                  continue
            # applies to the next pub fn, skipping the rest of its comment block,
            # attributes, and blank lines
            for la in range(ln + 1, min(ln + 30, len(raw_lines))):
                  s = raw_lines[la].strip()
                  fm = re.search(r"\bpub(\(crate\))?\s+(unsafe\s+)?fn\s+(\w+)", raw_lines[la])
                  if fm:
                        annotated[fm.group(3)] = m.group(1).strip()
                        break
                  if s and not s.startswith(("//", "#[", "#!")):
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
                             inputs=ins, output=ret, not_an_op=annotated.get(name),
                             in_place=inplace.get(name)))

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
# Gates: types, positional order (B*U*B*), mandatory trailing out block, and the
# plumbing inverted rule. The partition over TOTAL PARSED is a hard assert.
ALLOW = {
      "gpu_shutdown",        # device lifecycle, not a schedulable op
}
def is_plan_helper(r):
      # plan-time size query: no device work — only usize in, usize out
      return all(t == "usize" for t in r["inputs"]) and r["output"] == "usize" \
            and r["name"].endswith("_workspace_bytes")

def fn_body(mod, name):
      # body text of `pub fn name` in a module file (for the plumbing gate)
      src = open(os.path.join(SRC, mod + ".rs")).read()
      m = re.search(r"\bpub(\(crate\))?\s+(unsafe\s+)?fn\s+" + name + r"\b", src)
      if not m:
            return ""
      b0 = src.find("{", m.end())
      if b0 == -1:
            return ""
      depth, i = 0, b0
      while i < len(src):
            if src[i] == "{":
                  depth += 1
            elif src[i] == "}":
                  depth -= 1
                  if depth == 0:
                        break
            i += 1
      return src[b0:i]

if "--check" in sys.argv:
      total = len(rows)
      ffi = [r for r in rows if r["kind"] == "ffi"]
      plumb = [r for r in rows if r["kind"] == "plumbing"]
      all_op = [r for r in rows if r["kind"] == "op"]
      excluded = [r for r in all_op if r["not_an_op"]]
      allow_only = [r for r in all_op if r["name"] in ALLOW and not r["not_an_op"]]
      helpers = [r for r in all_op
                 if is_plan_helper(r) and not r["not_an_op"] and r["name"] not in ALLOW]
      gated = [r for r in all_op
               if not r["not_an_op"] and r["name"] not in ALLOW and not is_plan_helper(r)]

      viols = []
      for r in gated:
            why = []
            if r["recv"]:
                  why.append(f"method receiver {r['recv']}")
            for t in r["inputs"]:
                  if t not in ("&GpuBuffer", "usize"):
                        why.append(f"input {t}")
            if r["output"] != "Result<()>":
                  why.append(f"returns {r['output']}")
            cls = "".join("B" if t == "&GpuBuffer" else "U" for t in r["inputs"])
            if not re.fullmatch(r"B*U*B*", cls):
                  why.append(f"param order {cls} not (ins,dims,outs)")
            if not cls.endswith("B"):
                  why.append(f"no trailing out block ({cls}) — outs are mandatory, aliases included")
            if why:
                  viols.append((r, why))

      # plumbing inverted rule: exempt from the op constraint, but a pub fn there
      # that takes &GpuBuffer AND launches a kernel needs a not-an-op annotation.
      plumb_viols = []
      plumb_annotated = []
      for r in plumb:
            takes_buf = any("GpuBuffer" in t for t in r["inputs"])
            if not takes_buf:
                  continue
            if not re.search(r"\blaunch_\w+\s*\(", fn_body(r["module"], r["name"])):
                  continue
            if r["not_an_op"]:
                  plumb_annotated.append(r)
            else:
                  plumb_viols.append(r)

      for r, why in sorted(viols, key=lambda v: (v[0]["module"], v[0]["line"])):
            print(f"VIOLATION gpu-core/src/{r['module']}.rs:{r['line']} {r['name']}: {'; '.join(why)}")
      for r in plumb_viols:
            print(f"VIOLATION gpu-core/src/{r['module']}.rs:{r['line']} {r['name']}: plumbing pub fn takes &GpuBuffer and launches a kernel without not-an-op annotation")

      n_bad = len(viols) + len(plumb_viols)
      print(f"\n--check: {len(gated)} ops, {len(gated) - len(viols)} conforming, {n_bad} violating")

      # THE partition law over everything the parser saw. Off-by-one = crash.
      buckets = len(gated) + len(excluded) + len(helpers) + len(allow_only) + len(plumb) + len(ffi)
      assert buckets == total, \
            f"PARTITION BROKEN: {total} parsed != {len(gated)} gated + {len(excluded)} not-an-op " \
            f"+ {len(helpers)} auto-exempt + {len(allow_only)} allowlisted + {len(plumb)} plumbing + {len(ffi)} ffi = {buckets}"
      print(f"partition [asserted]: {total} parsed = {len(gated)} gated"
            f" + {len(excluded)} not-an-op + {len(helpers)} auto-exempt(usize-only *_workspace_bytes)"
            f" + {len(allow_only)} allowlisted + {len(plumb)} plumbing + {len(ffi)} ffi decls")
      from collections import Counter as _C
      pm = _C(r["module"] for r in plumb)
      print("plumbing per module: " + ", ".join(f"{m}={c}" for m, c in sorted(pm.items())))
      print(f"allowlisted: {sorted(r['name'] for r in allow_only) or '(empty)'}")
      print(f"auto-exempt helpers ({len(helpers)}):")
      for r in sorted(helpers, key=lambda r: (r["module"], r["line"])):
            print(f"  gpu-core/src/{r['module']}.rs:{r['line']} {r['name']}")
      if excluded:
            print(f"\nnot-an-op exclusions ({len(excluded)}) — audit this list:")
            for r in sorted(excluded, key=lambda r: (r["module"], r["line"])):
                  print(f"  gpu-core/src/{r['module']}.rs:{r['line']} {r['name']}: {r['not_an_op']}")
      if plumb_annotated:
            print(f"\nplumbing launch-fns accepted by annotation ({len(plumb_annotated)}):")
            for r in plumb_annotated:
                  print(f"  gpu-core/src/{r['module']}.rs:{r['line']} {r['name']}: {r['not_an_op']}")
      aliases = [r for r in all_op if r.get("in_place")]
      if aliases:
            print(f"\nOPDESC alias records (// in-place:, {len(aliases)}):")
            for r in sorted(aliases, key=lambda r: (r["module"], r["line"])):
                  print(f"  gpu-core/src/{r['module']}.rs:{r['line']} {r['name']}: {r['in_place']}")
      sys.exit(1 if n_bad else 0)
