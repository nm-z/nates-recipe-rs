# Audit lexer

This document describes the current implementation in
'audit/src/lexer.rs'. The lexer is a small, policy-oriented scanner. It is
not a parser for Rust, Zig, C, C++, LLVM IR, or any build-language grammar.
Its job is to remove comments, retain complete policy-relevant names and
strings, retain the '@' marker used by LLVM IR, and attach enough source
position information for findings.

## Boundary and call graph

'lex' is crate-private. The module itself is private in 'audit/src/lib.rs',
so callers reach it only through the source-audit path:

1. 'collect_native_scope' classifies supported files as a 'SourceKind', reads
   each file as UTF-8, and constructs a 'SourceUnit'
   ('audit/src/native.rs:168-229', 'audit/src/model.rs:182-209').
2. 'recipe_audit::audit' iterates the supplied source units
   ('audit/src/lib.rs:42-63').
3. 'audit_source' skips the self-hosted policy dictionary
   'audit/src/policy.rs'; every other source is passed to 'lex'
   ('audit/src/source.rs:16-29').
4. The returned lexemes feed the generic identifier/string checks and the
   LLVM declaration/call classifier ('audit/src/source.rs:31-107').

'audit_source' maps a 'LexError' to the public 'AuditError::Lexical', copying
the source path, one-based line, and static reason
('audit/src/source.rs:23-29'). 'audit' propagates that error with '?', so a
lexical failure returns no report and no partial source findings. The CLI
prints the error and returns exit code 2 ('audit/src/main.rs:10-19'). This is
different from a successful audit report containing blocking findings, which
the CLI reports with exit code 1.

The CLI collector admits only valid UTF-8 text. Direct library callers also
pass a Rust '&str', so the scanner can use byte offsets while preserving valid
UTF-8 when it copies token text.

The collector never sends unsupported extensions to this module and skips
'.git' and 'target' directories. A supported file that is not valid UTF-8
fails earlier as an 'AuditError::Io'; it is not a lexical error. Direct
callers can still construct a 'SourceUnit' for any 'SourceKind'.

At the public audit boundary, input validation runs before any source is
lexed ('audit/src/lib.rs:42-44'). Once validation succeeds, sources are
processed in the order supplied, then dependency, linker, and ELF facts are
evaluated ('audit/src/lib.rs:53-63'). A lexical error therefore stops that
sequence before later fact classes are evaluated; it cannot be converted into
a blocking finding or grandfathered by legacy grants.

For the CLI path, recursive directory entries and the collected path list are
sorted before 'SourceUnit' values are built ('audit/src/native.rs:168-203').
Thus the first lexical error from native collection is deterministic by
relative path. Library callers control source order directly, subject to the
unique-path validation.

## Data model

The types are defined at 'audit/src/lexer.rs:3-21'.

| Type or variant | Meaning |
| --- | --- |
| 'LexemeKind::Identifier' | One complete ASCII policy name. |
| 'LexemeKind::String' | A complete double-quoted literal, or a Rust raw string. Character literals are consumed but do not produce this variant. |
| 'LexemeKind::At' | A standalone '@', always emitted with text '"@"'. LLVM auditing uses it to identify an IR symbol that follows. |
| 'Lexeme.kind' | The variant above. |
| 'Lexeme.text' | Exact source text for identifiers; '@' for 'At'; literal contents without the outer delimiters for strings. Escapes are not decoded. |
| 'Lexeme.line' | One-based line on which the token's opening byte occurs. Multiline string tokens keep their opening line. There is no column field. |
| 'LexError.line' | One-based line on which the unterminated construct opened, not the line where end-of-input was reached. |
| 'LexError.reason' | One of the four static strings described under [failure behavior](#failure-behavior). |

Successful output is an ordered 'Vec<Lexeme>'. Tokens appear in source order;
whitespace, punctuation other than '@', standalone numeric bytes, comments,
character literals, Rust lifetimes, and every other unrecognized byte produce
no token. Digits that continue an identifier remain part of that identifier.
An empty or token-free input returns
'Ok(Vec::new())'.

For a compact machine-readable description of the returned values:

~~~text
Lexeme =
    Identifier { text: ASCII_IDENTIFIER, line: ONE_BASED_LINE }
  | String     { text: SOURCE_CONTENT,  line: ONE_BASED_LINE }
  | At         { text: "@",             line: ONE_BASED_LINE }

LexResult = Ok(ordered Lexeme*) | Err {
    line: ONE_BASED_LINE,
    reason: BLOCK | STRING | CHARACTER | RAW_STRING
}
~~~

The notation is descriptive rather than a second implementation. 'Lexeme'
and 'LexError' remain the Rust structures in the source file, and the error
reason strings are the exact literals listed under [failure
behavior](#failure-behavior).

## Function index

| Function | Current source | Role |
| --- | --- | --- |
| 'lex' | 'audit/src/lexer.rs:23-113' | Top-level byte loop, source-kind dispatch, token emission, and conversion of helper failure into one of the lexical errors. |
| 'push_identifier' | 'audit/src/lexer.rs:115-126' | Consume and copy one identifier. |
| 'is_identifier_start' | 'audit/src/lexer.rs:128' | Decide whether a byte can begin an identifier. |
| 'is_identifier_continue' | 'audit/src/lexer.rs:130-132' | Decide whether a byte extends an identifier. |
| 'skip_line_comment' | 'audit/src/lexer.rs:134-139' | Advance to LF or EOF without changing the line counter. |
| 'skip_block_comment' | 'audit/src/lexer.rs:141-161' | Consume nested block comments and count LF bytes. |
| 'normal_string_end' | 'audit/src/lexer.rs:163-176' | Find a closing double-quote or apostrophe while honoring backslash skips and counting unescaped LF bytes. |
| 'rust_raw_string_start' | 'audit/src/lexer.rs:178-189' | Recognize the Rust 'r' plus hash prefix and return content start/hash count. |
| 'rust_lifetime_end' | 'audit/src/lexer.rs:191-208' | Distinguish a Rust lifetime from an apostrophe literal and return the lifetime end. |
| 'rust_raw_string_end' | 'audit/src/lexer.rs:210-227' | Find the Rust raw close, return end/content boundaries, and count LF bytes. |

## Main scanner state

'lex' keeps three pieces of state ('audit/src/lexer.rs:23-29'):

| State | Invariant and use |
| --- | --- |
| 'bytes' | 'contents.as_bytes()'. All matching is on ASCII bytes. |
| 'index' | Current byte offset. Every successful branch advances it, and helper results point at the first byte not consumed. |
| 'line' | Starts at '1' and counts LF bytes. It is updated while comments and literals are skipped so later tokens retain their original line. |

The loop matches the following branches in this order
('audit/src/lexer.rs:29-109'). The order matters because comment and literal
scanners consume their bodies before their bytes can be considered for another
branch.

| Input at 'index' | Condition | Action and resulting tokens |
| --- | --- | --- |
| LF ('\n') | Any source kind | Increment 'line', advance one byte, emit nothing. |
| '//' | Any source kind | 'skip_line_comment' consumes through the byte before the next LF or through EOF. The LF is left for the main loop, which increments 'line'. |
| '/*' | Any source kind | 'skip_block_comment' consumes a nested block comment and updates 'line'; an absent close is a lexical error at the opening line. |
| ';' | 'kind == LlvmIr' | Treat the rest of the physical line as an LLVM comment. The LF remains to be counted by the main loop. |
| '#' | 'kind == BuildMetadata' | Treat the rest of the physical line as a build-metadata comment. A '#' in any other source kind is simply an unrecognized byte unless it is part of a Rust raw-string prefix handled by the 'r' branch. |
| '@' | Any source kind | Emit 'Lexeme { kind: At, text: "@", line }', then advance one byte. |
| '"' | Any source kind | Find a closing double quote with 'normal_string_end', emit one 'String' token, and set 'index' and 'line' to the end position. |
| Single quote | Any kind except 'BuildMetadata' | In Rust, first try 'rust_lifetime_end'; otherwise scan a character literal with 'normal_string_end' and emit no token. |
| 'r' | 'kind == Rust' | If followed by zero or more '#' bytes and then '"', scan one raw string. Otherwise scan 'r' as an identifier. |
| Identifier-start byte | Any source kind | 'push_identifier' consumes the complete identifier and emits one 'Identifier'. |
| Anything else | Any source kind | Advance one byte and emit nothing. |

Comment markers and quote characters inside a recognized string are handled
by the literal helper, not by the top-level loop. Conversely, a comment marker
outside a string always wins over identifier or punctuation handling.

## Identifier grammar used by the scanner

'push_identifier', 'is_identifier_start', and 'is_identifier_continue' are
at 'audit/src/lexer.rs:115-132'.

* A start byte is an ASCII letter, '_', '$', or '.'.
* A continuation byte is an ASCII letter or digit, '_', '$', '.', or '-'.
* A hyphen is therefore allowed only after a valid start. 'foo-bar' is one
  identifier; '-foo' has no token for the leading hyphen and then produces
  'foo'.
* Digits cannot start an identifier. In '123cuInit', the digits are ignored
  and 'cuInit' is emitted.
* Unicode bytes are not identifier bytes. Outside literals they are skipped
  one byte at a time. Names are intentionally ASCII-oriented because policy
  classification operates on exact native spellings.
* '@' is never part of an identifier. LLVM spelling such as '@cuInit'
  becomes 'At("@")' followed by 'Identifier("cuInit")'.

'push_identifier' starts at a known ASCII boundary and ends at another
ASCII boundary, so its byte slice is a valid 'str' slice. It copies the
exact source substring and records the line at which the first byte appeared.

The scanner does not validate language-specific identifier rules. For
example, '$', '.', and hyphenated names are accepted for every source kind,
because the audit policy needs complete, stable tokens rather than a
language parser.

## Ownership and cost

The scanner owns the returned token text. Every emitted identifier, string,
and '@' marker stores a separate 'String'; the input 'contents' is borrowed
only for the duration of the scan. Characters that are skipped or consumed by
comments and literals are not retained in a side buffer.

Each top-level byte is consumed once, either by the main loop or by the
helper selected at that byte. The scan is therefore linear in the input byte
length, with additional copying proportional to the total length of emitted
token text. Memory held after return is the vector of lexemes and those owned
strings; there is no retained lexer state.

## Determinism

For fixed UTF-8 contents and one fixed 'SourceKind', 'lex' has no external
inputs, mutable global state, filesystem access, or policy lookup. The same
bytes therefore produce the same ordered lexemes or the same lexical error.
Determinism of the complete audit additionally depends on the caller's source
order and on the native collector's path sort; findings are sorted and
deduplicated downstream.

## Byte-boundary and arithmetic invariants

The input type guarantees valid UTF-8, but the scanner deliberately reasons
in bytes because every recognized boundary is ASCII. Identifier slices begin
and end on ASCII bytes. Regular-string slices begin after an ASCII quote and
end before an ASCII quote. Raw-string slices begin after the ASCII prefix and
end before an ASCII closing quote. Those boundaries keep the copied ranges
valid UTF-8 even when their contents contain multibyte characters.

Lookahead uses 'bytes.get' for comment, raw-prefix, lifetime, and raw-close
checks, so an absent lookahead byte is a non-match rather than an indexing
panic. The escaped-byte increment and the initial lifetime cursor advance use
'checked_add'; overflow makes the helper return 'None', which follows the
caller’s non-match or lexical-error path. Block-comment depth uses
'checked_add' for the same fail-closed behavior.

The scan has no separate offset or column state. The byte index is advanced
by the main loop or returned directly by each helper, and the only position
state retained in a lexeme is the one-based opening line.

## Comments and line accounting

### Line comments

'skip_line_comment' ('audit/src/lexer.rs:134-139') starts after the comment
marker and stops at the next LF or at 'bytes.len()'. It does not increment
'line' itself. The main loop sees a remaining LF and increments once. A
comment that reaches EOF is valid and produces no error.

The following markers are line comments:

* '//' for every 'SourceKind'.
* ';' only for 'LlvmIr'.
* '#' only for 'BuildMetadata'.

These markers are recognized only when the top-level loop is at the marker.
Markers inside regular or raw strings are literal text.

### Nested block comments

'skip_block_comment' ('audit/src/lexer.rs:141-161') starts with 'depth = 1'
after the opening '/*'. It then:

1. increments 'line' for each LF;
2. increments 'depth' for each nested '/*';
3. decrements 'depth' for each '*/';
4. returns immediately after the close that makes 'depth == 0'.

Nested block comments are accepted for all source kinds, even where the
underlying language would not normally allow nesting. An unmatched outer or
nested comment reaches EOF and returns 'None'; an extremely deep nesting
overflow from 'checked_add' also returns 'None'. Both cases become
'"unterminated block comment"' at the original opening line. Text in a block
comment is never tokenized, and comment markers inside the block are not
interpreted as strings.

### What counts as a line

Only LF ('\n') increments the counter. CRLF therefore advances one line at
the LF, while a standalone '\r' does not. A newline consumed by the escape
branch of 'normal_string_end' is skipped together with the backslash and is
not counted. The scanner has no special treatment for Unicode line
separators.

## Regular strings and character literals

'normal_string_end' is shared by double-quoted strings and non-lifetime
apostrophe literals ('audit/src/lexer.rs:163-176'). Starting immediately
after the opening delimiter, it scans until the matching delimiter:

* A backslash advances by two bytes, so the next byte cannot close the
  literal. This is escape recognition only; the bytes are not decoded.
* A matching delimiter returns its byte-after-close index and current line.
* An LF increments 'line' and is otherwise consumed as literal content.
* Any other byte advances by one.
* EOF returns 'None'. A trailing backslash also returns 'None' through
  'checked_add(2)'.

For a double-quoted string, 'lex' copies
'contents[opening + 1 .. closing]' into a 'String' lexeme and records the
opening line. The outer quotes are excluded, while all inner bytes,
including backslashes and newlines, remain unchanged. This scanner therefore
accepts multiline and otherwise syntactically invalid strings as long as a
matching delimiter exists. It is deliberately not a syntax validator.

For a non-build apostrophe, the same helper is used with a single-quote
delimiter, but the completed character literal is discarded rather than
emitted. Its only observable effect is consuming the literal and updating
the line counter. An unterminated one returns
'"unterminated character literal"' at its opening line.

'BuildMetadata' does not enter the apostrophe branch. Apostrophes in a
build-metadata file are ordinary ignored bytes, so the letters between them
can still form identifiers. A quoted single-quote construct in that source
kind is not a character-literal boundary.

## Rust-only handling

### Lifetimes

'rust_lifetime_end' ('audit/src/lexer.rs:191-208') is called only for a Rust
apostrophe. It recognizes an apostrophe followed by an ASCII letter or '_',
then consumes ASCII alphanumeric and '_' bytes. If the next byte after that
name is not another apostrophe, it returns the cursor and 'lex' skips the
whole lifetime without emitting a token.

If the name is immediately followed by another apostrophe, the helper returns
'None' so the same apostrophe is handled as a character literal. This
distinguishes a one-codepoint literal such as "'x'" from a lifetime such as
"'static". Longer quoted text such as "'abc'" is likewise routed to the
character-literal scanner. The scanner does not check whether the resulting
literal is valid Rust; it only requires a closing apostrophe.

An apostrophe followed by a non-ASCII byte, digit, or other punctuation is
not a lifetime candidate and goes directly to the character-literal path.
The helper does not inspect parser context, so a quote-like sequence in a
comment or string never reaches it.

### Raw strings

'rust_raw_string_start' ('audit/src/lexer.rs:178-189') recognizes only the
Rust 'r' form. At an 'r', it counts consecutive '#' bytes and requires the
next byte to be '"'. It returns the first content byte and the hash count.
Examples of recognized prefixes are 'r"..."', 'r#"..."#', and
'r##"..."##'. A prefix such as 'r#name' is not a raw string and is scanned as
identifier 'r', ignored '#', and subsequent tokens.

'rust_raw_string_end' ('audit/src/lexer.rs:210-227') scans from the content
start. It increments 'line' for each LF. A quote closes when the following
slice of length 'hashes' consists of '#' bytes; the implementation does not
require that the run end there, so a longer run also closes after the
required prefix. The returned content end is the quote byte, so the closing
quote and the required hashes are excluded from token text. The emitted
'String' token contains the raw contents exactly, including quotes,
backslashes, comment markers, and newlines that occur before the matching
close. Its line is the opening line.

No Rust raw byte-string form ('br"..."' or 'br#"..."#') is recognized as a
single raw token. The leading 'br' is an ordinary identifier and the
following quote starts a normal double-quoted string. This is an intentional
consequence of the small scanner, not a complete Rust lexical grammar.

An unclosed raw string returns '"unterminated raw string literal"' at its
opening line. A quote with too few following hashes is content and does not
terminate the scan. A quote with extra hashes satisfies the current
at-least-'hashes' check and closes after the requested number.

## Source-kind matrix

'SourceKind' is declared in 'audit/src/model.rs:182-191', and the native
collector selects it from file extensions and well-known build filenames
('audit/src/native.rs:205-226').

| 'SourceKind' | Collector inputs | Lexical additions or differences |
| --- | --- | --- |
| 'Rust' | '.rs' | Generic comments, double strings and apostrophe literals, Rust lifetime skipping, and Rust 'r' raw strings. ';' and '#' are ordinary ignored bytes. |
| 'Zig' | '.zig' | Generic comments, double strings, and apostrophe literals. No Rust lifetime or raw-string handling; ';' and '#' are not comment markers. |
| 'C' | '.c', '.h' | Same generic behavior as Zig. |
| 'Cpp' | '.cc', '.cpp', '.cxx', '.hh', '.hpp', '.hxx' | Same generic behavior as Zig. |
| 'LlvmIr' | '.ll' | Generic comments and literals, plus ';' line comments. '@' tokens are later interpreted by 'audit_llvm' as possible declaration/call symbols. |
| 'BuildMetadata' | '.toml', '.json', '.yaml', '.yml', '.cmake', '.mk', '.ninja', '.bazel', '.bzl', 'Cargo.lock', 'Makefile', 'CMakeLists.txt', 'BUILD', 'BUILD.bazel', 'WORKSPACE', 'WORKSPACE.bazel' | Generic '//' and nested '/* */' comments, plus '#' line comments. Apostrophes are ignored rather than scanned as character literals. Rust raw-string and lifetime handling is disabled. |

The matrix describes scanner behavior, not complete syntax support. For
example, JSON strings are retained by the double-quote branch, but the lexer
does not validate JSON escapes or commas. Build metadata is later checked for
both interface names and prohibited dependency/library names.

## Downstream token consumption

'audit_source' uses the token stream in two passes.

### Generic identifiers and strings

For every 'Identifier', it calls 'push_interface_finding' with the token's
exact text and line. Build metadata changes the normal category to
'BuildLinkInput'; other source kinds use 'SourceApi'
('audit/src/source.rs:37-50', 'audit/src/source.rs:194-200'). Policy
classification is exact over the complete identifier, which is why the lexer
must not emit substrings from comments or split a hyphenated name.

'push_interface_finding' first classifies the complete text as an interface.
For build metadata only, an unknown interface classification is then offered
to dependency classification. Allowed and unknown results produce no finding.
Prohibited CUDA Driver symbols outside the reviewed allowlist and prohibited
direct KFD symbols use 'DisallowedNativeInterface'; every other prohibited
symbol uses 'BuildLinkInput' for build metadata or the source-kind category
selected above. This category decision is downstream of lexing, but it relies
on the lexer's complete-token and exact-line guarantees.

For every 'String', it calls 'audit_string'
('audit/src/source.rs:51-55', 'audit/src/source.rs:110-139'). The consumer
trims surrounding whitespace, then removes at most one trailing '\00' suffix
and at most one trailing '\0' suffix, in that order. It classifies the
complete trimmed string and then its components split on slash, backslash,
equals, comma, colon, parentheses, brackets, semicolon, and ASCII whitespace.
The lexer's literal text is therefore intentionally undecoded source text.

For a non-build source, string classification checks prohibited libraries and
interface symbols. For 'BuildMetadata', it also checks prohibited dependency
names, both for the complete value and for each nonempty component. A string
that is on link context is classified as build metadata even when the
underlying 'SourceKind' is Rust, C, C++, Zig, or LLVM IR.

'At' has no generic action. It exists so the LLVM-specific pass can recognize
IR symbol references.

### LLVM declarations and calls

For 'LlvmIr', 'audit_llvm' scans every 'At' and pairs it with the immediately
following 'Identifier' or 'String' token, regardless of whether punctuation
was between them. It looks backward only across identifier tokens on the
'At' token's line. If that prefix contains 'declare', the finding category is
'LlvmDeclaration'; otherwise 'call', 'invoke', or 'callbr' selects
'LlvmCall'. Without either marker, the '@' pair produces no LLVM finding
('audit/src/source.rs:66-107').

The generic pass skips an LLVM identifier or string only when the immediately
preceding token is 'At' on the same line ('follows_at',
'audit/src/source.rs:37-57', '202-204'). Consequently, a symbol after a
newline can still be paired by 'audit_llvm', while the generic pass will not
skip it. This is a downstream line-sensitive rule made possible by the
lexer's 'line' field.

The line on a multiline string is its opening line, so all source-context
checks in 'audit_string' and the final finding line use that opening line.
'line_text' then indexes the original source with one-based line numbers
('audit/src/source.rs:206-234').

After both passes, 'audit_source' sorts and deduplicates its findings before
returning them ('audit/src/source.rs:61-63'). Thus two lexical routes that
identify the same category, path, line, and symbol collapse to one result;
the lexer itself never deduplicates tokens.

The source-context helpers consume that same lexer line:

* 'line_has_link_context' searches the original line for
  'rustc-link-lib', 'rustc-link-arg', '#[link', 'target_link_libraries',
  'linkSystemLibrary', or '-l'. When 'audit_string' receives a string on such
  a line, it treats it as build metadata and emits 'BuildLinkInput' for a
  prohibited value.
* 'line_has_include_context' searches the original line for '#include',
  '@import', or '@cImport'. When link context is absent, this selects
  'SourceApi'; a prohibited string with neither context is 'RuntimeLoad'.
* 'line_text' converts the one-based lexer line with checked subtraction and
  indexes 'source.contents.lines()'. A line outside that iterator yields no
  context, even if a token was produced by unusual line-ending or escaped
  newline handling.

These checks are why a token's opening line, rather than its closing line, is
part of the lexer contract. The lexer does not inspect the context markers;
it only preserves the line needed by the source consumer.

## Invariants and non-goals

The implementation maintains these observable invariants:

* 'line' is one-based and advances only for LF bytes consumed by the main
  loop, block-comment scanner, raw-string scanner, or the unescaped path of
  'normal_string_end'.
* A successful helper returns an index after its complete construct:
  'skip_line_comment' stops at LF or EOF, block comments stop after '*/',
  regular literals stop after their delimiter, raw strings stop after their
  quote and hashes, and identifiers stop before the first non-continuation
  byte.
* A token's text is a copied source slice, never a normalized or decoded
  spelling. The only synthetic text is '"@"'.
* Comments, strings, and character/lifetime constructs are opaque to the
  top-level token matcher while they are being consumed.
* The output contains no whitespace, comment, punctuation, numeric, lifetime,
  or character-literal tokens.
* Lexing is a forward scan with no backtracking and no parser state. It
  recognizes only the complete atoms needed by policy and LLVM context.

The lexer does not validate balanced parentheses, operators, numeric
syntax, escape validity, language-specific identifiers, or the grammatical
placement of a token. It does not decode escapes, report columns, or expose
partial output after an error. Those are deliberate boundaries: policy uses
the exact names and strings that survive this scan, while syntax and build
validity remain outside this module.

## Failure behavior

There are four lexical error reasons, all created in 'lex':

| Reason | Trigger | Reported line |
| --- | --- | --- |
| 'unterminated block comment' | EOF before nested comment depth reaches zero, or block-depth overflow through 'checked_add' | Line where the outer '/*' opened |
| 'unterminated string literal' | EOF before a closing '"', including a trailing backslash that cannot skip two bytes | Line where the '"' opened |
| 'unterminated character literal' | Non-build apostrophe is not classified as a Rust lifetime and reaches EOF without a closing single quote | Line where the apostrophe opened |
| 'unterminated raw string literal' | Rust raw prefix was recognized but no quote plus the required number of hashes appears before EOF | Line where the 'r' opened |

No line comment can fail at EOF. A failure in any helper is converted
immediately with '?', so 'lex' returns 'Err(LexError)' instead of its
accumulated token vector. 'audit_source' then returns
'AuditError::Lexical { path, line, reason }', whose display form is
'{path}:{line}: lexical audit failed: {reason}'
('audit/src/error.rs:33-45').

## Representative token traces

The following traces use the exact current rules. 'Identifier(text, line)',
'String(text, line)', and 'At("@", line)' abbreviate 'Lexeme' values.

| Source kind and input | Result |
| --- | --- |
| 'Rust', 'cuInit foo-bar' | 'Identifier("cuInit", 1)', 'Identifier("foo-bar", 1)' |
| 'Rust', '"cuInit // not a comment"' | 'String("cuInit // not a comment", 1)' |
| 'Rust', 'r#"cuInit /* text */"#' | 'String("cuInit /* text */", 1)' |
| 'Rust', "'static cuInit" | No token for "'static"; then 'Identifier("cuInit", 1)' |
| 'Rust', "'x' cuInit" | The character literal is consumed; then 'Identifier("cuInit", 1)' |
| 'LlvmIr', 'declare i32 @cuInit()' | 'Identifier("declare", 1)', 'Identifier("i32", 1)', 'At("@", 1)', 'Identifier("cuInit", 1)' |
| 'LlvmIr', 'cuInit ; @ignored' | 'Identifier("cuInit", 1)'; the semicolon consumes the rest of the line |
| 'BuildMetadata', '# ignored\nhip = "cudart"' | 'Identifier("hip", 2)', 'String("cudart", 2)' |
| Any kind, '/* outer\n /* inner */\n */cuInit' | 'Identifier("cuInit", 3)' |
| Any kind, '/* missing' | 'Err(line: 1, reason: "unterminated block comment")' |
| 'Rust', 'r#"missing' | 'Err(line: 1, reason: "unterminated raw string literal")' |

These examples describe lexer output only. Whether a token becomes a
blocking finding depends on the source kind, source line context, and policy
classification in 'audit/src/source.rs' and 'audit/src/policy.rs'.
