export const meta = {
  name: 'clippy-zero-v3',
  description: 'FFI-layer campaign: stage-0 cast helpers + h03 revival, then verify/fix loop with chunked edits-as-data for monster files',
  phases: [{ title: 'Stage0', detail: 'cast helper family + hygiene h03 revival' }, { title: 'Verify', detail: 'serial fmt+clippy census, chunk-split big files' }, { title: 'Fix', detail: 'owner-fixers for small files; computers+applier for big ones' }],
}
const S = '/tmp/claude-1000/-home-nate-Desktop-nates-recipe-rs/87c917cb-bfc1-486b-9cfb-39e5d28b903a/scratchpad'
const R = '/home/nate/Desktop/nates-recipe-rs'
const GEN_SCHEMA = { type: 'object', required: ['file', 'changed', 'notes'], properties: { file: { type: 'string' }, changed: { type: 'array', items: { type: 'string' } }, notes: { type: 'string' } } }
const VERIFY_SCHEMA = {
  type: 'object',
  required: ['total_errors', 'files', 'diag_prefix', 'notes'],
  properties: {
    total_errors: { type: 'integer' },
    files: { type: 'array', items: { type: 'object', required: ['file', 'count', 'chunks'], properties: { file: { type: 'string' }, count: { type: 'integer' }, chunks: { type: 'integer' } } } },
    diag_prefix: { type: 'string' },
    notes: { type: 'string' },
  },
}
const FIX_SCHEMA = {
  type: 'object',
  required: ['file', 'fixed_count', 'per_class', 'handoff'],
  properties: { file: { type: 'string' }, fixed_count: { type: 'integer' }, per_class: { type: 'array', items: { type: 'string' } }, handoff: { type: 'string' } },
}
const COMPUTE_SCHEMA = { type: 'object', required: ['edits_path', 'computed', 'skipped'], properties: { edits_path: { type: 'string' }, computed: { type: 'integer' }, skipped: { type: 'array', items: { type: 'object', required: ['site', 'why'], properties: { site: { type: 'string' }, why: { type: 'string' } } } } } }
const APPLY_SCHEMA = { type: 'object', required: ['file', 'applied', 'failed', 'notes'], properties: { file: { type: 'string' }, applied: { type: 'integer' }, failed: { type: 'integer' }, notes: { type: 'string' } } }
const COMMON = `House law (2026-07-15):
- Comments: /// and //! doc comments unlimited (missing_errors_doc fixes WRITE the "/// # Errors" section — one terse line on when it errors). "// SAFETY:" REQUIRED where undocumented_unsafe_blocks demands, exactly 1 per unsafe block, one line stating why the invariants hold. Any other // comment: max 1 per function. No block comments, no dividers.
- FFI casts (owner spec, verbatim): use c_int if the FFI signature is C int; use i32 only if the extern decl already says i32; if FFI expects u32, use the separate cu() helper; nonzero check is SEPARATE from range check — never fold nonzero semantics into a cast. Use the gpu-core helper family (stage 0 creates it); launchers return Result so checked casts bubble.
- Raw-pointer as-casts: .cast::<T>() / .cast_const() / .cast_mut().
- Campaign-added #[non_exhaustive] attributes are MOOT (owner allowed exhaustive_structs) — remove them on sight in your file; keep the new()/constructors that now have callers.
- Explicit return on every tail including closures/match arms. No unwrap/expect/panic outside tests. drop(expr) for must_use discards, never let-underscore. Never create a fn with exactly one call site. Frozen: scanner patterns/messages, log public API, test assertions byte-for-byte, kernel math.
- Never run cargo. Never run git. Only structural fixes survive cargo fmt. No allow attributes, no lint-config edits.`
phase('Stage0')
const stage0 = await parallel([
  () => agent(`In ${R}/gpu-core, create the FFI cast helper family the campaign needs (~826 usize-to-C-integer call sites will adopt it). FIRST survey ground truth with rg (timeouts): what integer types do the extern "C" launch_* decls actually take — core::ffi::c_int/libc::c_int, literal i32, u32? Then add pub(crate) helpers in ONE existing well-imported module (hip.rs or kernels.rs top — your call from where the error type lives): a c_int range-checked helper, an i32 one ONLY if extern decls literally say i32, a cu() for u32 takers. Each returns the crate's existing launcher error type with a message naming the offending value (fail-clean, no clamping, no panic). NO nonzero checking inside them. To avoid dead_code between now and adoption, convert 2-3 real call sites per helper yourself as anchors. ${COMMON} Return JSON: file, changed, notes (helper names + signatures + where they live — fixers will call them).`, { label: 's0:cast-helpers', phase: 'Stage0', model: 'opus', effort: 'high', schema: GEN_SCHEMA }),
  () => agent(`Create ${R}/tests/hygiene.rs reviving h03 under the NEW budgeted-comment law: /// and //! exempt entirely; "// SAFETY:" exempt but max 1 per unsafe block; any other // comment max 1 per FUNCTION; block comments banned outright. Lexer-based scan (strip string/char/raw-string contents before matching — the old h03 used a mark() lexer; reconstruct equivalent logic) over committed workspace sources: all members' src/ plus root and gpu-core build.rs. The test FAILS listing file:line of each violation. Structure per repo test law: tests/hygiene.rs contains only the cfg(test)+path-attributed module declaration pointing at tests/hygiene/cases.rs where the test fn lives (same pattern as ogdl/tests/*.rs — read one for the exact form). The test itself must satisfy every deny lint (explicit returns; tests may use expect per clippy.toml but prefer not). It must pass on a law-abiding tree and fail on a violating one — no test that cannot fail. ${COMMON} Return JSON: file, changed, notes.`, { label: 's0:h03-revival', phase: 'Stage0', model: 'opus', effort: 'high', schema: GEN_SCHEMA }),
])
log('stage0 done: ' + stage0.filter(Boolean).length + '/2')
const helperNotes = (stage0[0] && stage0[0].notes) || 'helpers in gpu-core (rg for ci/cu fns)'
function verifyPrompt(round) {
  return `Serial verifier, round ${round}, in ${R}. You MAY run exactly: timeout 120 cargo fmt; timeout 900 cargo clippy --workspace --all-targets 2>${S}/v3r${round}.log; perl/node with timeout for splitting. Split rendered error blocks (starting with error at line start, excluding could-not-compile summaries) by their --> file. For files with more than 80 blocks, write chunk files ${S}/v3r${round}_<path-underscored>_c<k>.txt of at most 40 blocks each (k from 0); for smaller files one file ${S}/v3r${round}_<path-underscored>_c0.txt. Return JSON: total_errors, files=[{file (repo-relative), count, chunks}] sorted desc, diag_prefix="${S}/v3r${round}_", notes (fmt status, newly-linted crates, hard rustc errors, biggest classes).`
}
const APPLIER_CODE = `const fs=require("fs");const R="${R}/";const paths=process.argv.slice(2);const edits=paths.flatMap(p=>JSON.parse(fs.readFileSync(p,"utf8")).edits);const files={};const getF=f=>files[f]=files[f]||fs.readFileSync(R+f,"utf8").split("\\n");let ok=0;const fails=[];edits.sort((a,b)=>b.line-a.line);for(const e of edits){const L=getF(e.file);const cur=L[e.line-1];if(cur!==e.old){if(e.new!==undefined&&e.new!==null&&cur===e.new){ok++;continue}fails.push(e.site+" cur="+JSON.stringify(cur));continue}const ins=e.insert_before||[];const rep=(e.new===undefined||e.new===null)?[cur]:[e.new];L.splice(e.line-1,1,...ins,...rep);ok++}for(const[f,L]of Object.entries(files))fs.writeFileSync(R+f,L.join("\\n"));console.log("applied="+ok+" failed="+fails.length);fails.forEach(x=>console.log("FAIL "+x));`
function computePrompt(file, diag) {
  return `READ-ONLY computer for ${R}/${file}: do not edit any repo file, no cargo, no git. Read the diagnostics at ${diag} (rendered clippy blocks with suggestions) and the current file, and compute the fix for EVERY block as edit records. Cast-helper context from stage 0: ${helperNotes}

${COMMON}

Write your records as JSON to a NEW scratchpad file (under ${S}/, named after your diag chunk) with shape {"edits":[{"file":"${file}","line":N,"old":"<current full line, real tabs>","new":"<replacement line, or null if only inserting>","insert_before":["<full lines to insert above, e.g. a SAFETY line or doc lines>"],"site":"<lint>:<line>"}],"skipped":[]}. One record per touched line; multi-line insertions ride insert_before on their anchor; never emit a record whose old you did not verify against the file. Return JSON per the schema: edits_path, computed, skipped.`
}
function applyPrompt(file, editPaths) {
  return `Applier for ${R}/${file}. Edit-record JSONs from read-only computers: ${editPaths.join(' ')}. Write this EXACT node script to ${S}/apply.js (overwrite fine) and run it with the edit paths as arguments (timeout 60): ${JSON.stringify(APPLIER_CODE)}. It applies bottom-up with old-line verification and prints applied/failed counts plus FAIL lines. For each FAIL line, fix that site MANUALLY per its original diagnostic (read the file fresh — a neighboring edit likely shifted it). You may edit ${R}/${file} only. No cargo, no git. Return JSON: file, applied, failed (after manual completion; 0 unless truly impossible), notes.`
}
function fixPrompt(file, diag) {
  return `You own ${R}/${file}. Fix EVERY clippy error in the diagnostics at ${diag}. Prior agents may have edited this file — work from CURRENT state. Cast-helper context: ${helperNotes}

${COMMON}

Return JSON: file, fixed_count, per_class, handoff.`
}
const MAX_ROUNDS = 8
let prev = Infinity
let stall = 0
const history = []
for (let round = 1; round <= MAX_ROUNDS; round++) {
  phase('Verify')
  const v = await agent(verifyPrompt(round), { label: 'v3verify:r' + round, phase: 'Verify', model: 'opus', effort: 'medium', schema: VERIFY_SCHEMA })
  if (!v) { history.push({ round, error: 'verifier died' }); break }
  log(`round ${round}: ${v.total_errors} errors in ${v.files.length} files`)
  history.push({ round, total: v.total_errors, files: v.files.map(f => f.file + '(' + f.count + ')'), notes: v.notes })
  if (v.total_errors === 0) { return { converged: true, rounds: round, history } }
  if (v.total_errors >= prev) { stall++ } else { stall = 0 }
  if (stall >= 2) { return { converged: false, reason: 'stalled', history } }
  prev = v.total_errors
  phase('Fix r' + round)
  const results = await parallel(v.files.filter(f => f.count > 0).map(f => () => {
    const und = f.file.replace(/[/.]/g, '_')
    if (f.count <= 80) {
      return agent(fixPrompt(f.file, v.diag_prefix + und + '_c0.txt'), { label: 'v3r' + round + ':' + f.file.split('/').pop(), phase: 'Fix r' + round, model: 'opus', effort: f.count > 15 ? 'high' : 'medium', schema: FIX_SCHEMA })
    }
    const chunkIdx = Array.from({ length: f.chunks }, (x, k) => k)
    return parallel(chunkIdx.map(k => () =>
      agent(computePrompt(f.file, v.diag_prefix + und + '_c' + k + '.txt'), { label: 'v3r' + round + ':' + f.file.split('/').pop() + ':c' + k, phase: 'Fix r' + round, model: 'opus', effort: 'medium', schema: COMPUTE_SCHEMA })
    )).then(cs => {
      const paths = cs.filter(Boolean).map(c => c.edits_path)
      if (!paths.length) { return null }
      return agent(applyPrompt(f.file, paths), { label: 'v3r' + round + ':apply:' + f.file.split('/').pop(), phase: 'Fix r' + round, model: 'opus', effort: 'high', schema: APPLY_SCHEMA })
    })
  }))
  history[history.length - 1].fixed = results.filter(Boolean).length
}
return { converged: false, reason: 'round cap', history }