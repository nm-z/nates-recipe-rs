# Decimal payload contract

[`ingest/src/numeric.rs`](../../src/numeric.rs) is the one lexical admission
boundary for decimal text that will become a calculation-facing `f32` or
`i32` payload. It does not normalize a column, impute missing values, infer a
semantic type, or perform a GPU calculation. It proves that a source decimal
has a bounded declared precision and that the selected payload preserves that
precision before the caller stores the payload bits.

The module is private inside `recipe-ingest`; its public constants, error
types, payload wrappers, and parser functions are re-exported by
[`ingest/src/lib.rs`](../../src/lib.rs). The internal `DecimalResult<T>` alias
is not re-exported, so the public signatures are effectively
`Result<F32Decimal, DecimalError>` and `Result<I32Decimal, DecimalError>`.

## Public surface

| Item | Contract |
| --- | --- |
| `F32_GUARANTEED_SIGNIFICANT_DIGITS` | `6`. A decimal accepted by `parse_contract_f32` has between one and six significant digits according to the scanner below. |
| `I32_GUARANTEED_SIGNIFICANT_DIGITS` | `9`. A decimal accepted by `parse_contract_i32` has between one and nine significant digits. |
| `DecimalErrorKind` | Non-exhaustive classification of an admission failure: `Empty`, `InvalidSyntax`, `TooManySignificantDigits`, `OutsidePayloadRange`, or `PrecisionLoss`. |
| `DecimalError` | Owns the `kind`, the exact original `input`, and a human-readable `detail`. Its `Display` form is `Kind: detail for "input"`. |
| `F32Decimal` | Private `u32` IEEE-754 binary32 bits plus a nonzero validated significant-digit count. |
| `I32Decimal` | Private exact `i32` value plus a nonzero validated significant-digit count. |
| `parse_contract_f32` | Scans a floating decimal, proves the six-digit f32 boundary, and returns `F32Decimal`. |
| `parse_contract_i32` | Scans an integer decimal, enforces the nine-digit boundary, and returns `I32Decimal`. |

Both payload wrappers derive `Clone`, `Copy`, `Debug`, `PartialEq`, and `Eq`.
Their fields are private, so callers cannot construct a wrapper without going
through the parser's proof. The wrappers are values, not views into the source
string. `DecimalError` is also `Clone`, `Debug`, `PartialEq`, and `Eq`, and its
kind is `#[non_exhaustive]`; external matches must include a wildcard arm.

### `F32Decimal`

`bits()` returns the exact `u32` bit pattern. `value()` reconstructs the
`f32` with `f32::from_bits`, so signed zero and every finite subnormal bit
pattern survive. `significant_digits()` returns a `NonZeroU8` in `1..=6`.
`to_le_bytes()` returns the four little-endian bytes of `bits()`.

`decimal_round_trip()` formats `value()` as scientific notation with exactly
`significant_digits - 1` digits after the decimal point. It is canonical
notation at the proven input precision, not a reproduction of the input's
spelling: leading zeroes, a plus sign, a decimal-point choice, and exponent
spelling are not retained. The method is useful for displaying the precision
that was proven. The wrapper's equality includes both the bits and the digit
count, so two spellings that produce the same bits but declare different
precision are different `F32Decimal` values.

### `I32Decimal`

`value()` returns the exact signed `i32`. `significant_digits()` returns a
`NonZeroU8` in `1..=9`, and `to_le_bytes()` returns the value's four
little-endian bytes. Equality includes the digit count as well as the integer
value. No decimal spelling or round-trip string is retained.

`NonZeroU8` is an invariant, not a user-controlled option. The scanner reports
at least one digit even when the mantissa is all zero, and the parser checks
the limit before constructing the wrapper.

## Source representation and missing values

Table framing in [`ingest/src/table.rs`](../../src/table.rs) stores headers and
cells as `Vec<u8>`. A `RawTable` is therefore a rectangular
`Vec<Vec<Vec<u8>>>`; framing performs no type detection, trimming, numeric
conversion, scaling, or imputation. An empty cell (`value.is_empty()`) is the
missing-value marker used by semantic inference and preparation. It is not
passed to either decimal parser.

The parsers take `&str`, not bytes. Every caller that starts with a table cell
must first call `core::str::from_utf8`; invalid UTF-8 is consequently either an
outer preparation error or, in semantic inference, evidence that the column is
not numeric. It is never a `DecimalError`. Whitespace is never trimmed by the
numeric layer. A space, tab, newline, non-ASCII whitespace byte, comma,
underscore, or any other separator remains in the input and is rejected by
the scanner.

The payload contract is about decimal text only. Binary image headers,
temporal strings, categorical labels, ordinal labels, and arbitrary text use
their own semantic parsers and do not become numeric merely because their
bytes happen to be present in a table.

## Lexical grammar

`scan_decimal` performs a byte-level scan before any standard-library numeric
conversion. The grammar below describes the scanner's accepted shape. The
subsequent `str::parse` call can still reject a syntactically valid value for
payload range or implementation conversion reasons.

```text
sign      := "+" | "-"
digit     := "0" .. "9"

integer   := [sign] digit+

float     := [sign] (digit+ ("." digit*)? | "." digit+) [exponent]
exponent  := ("e" | "E") [sign] digit+
```

The notation uses brackets for an optional part and `+` for one or more
occurrences. Thus a float must contain at least one mantissa digit, but it may
be written with a leading point (`.5`) or a trailing point (`5.`). An integer
has no point and no exponent. A leading sign is optional and is allowed only
once, at the start; a sign after `e` or `E` is allowed only for a float
exponent. The scanner accepts upper- or lower-case `e`, but only one exponent
marker because it stops at the first marker and requires the rest to be one
optional sign followed by digits.

The scanner rejects:

* an empty string or a string containing only `+` or `-`;
* a mantissa with no digit, such as `.` or `e10`;
* a second decimal point, an exponent in integer mode, or a malformed exponent
  such as `1e`, `1e+`, or `1e2.0`;
* whitespace, separators, alphabetic spellings such as `NaN` and `inf`, and
  all bytes outside the ASCII forms above.

These failures return `DecimalErrorKind::Empty` only for the empty string. All
other scanner failures return `DecimalErrorKind::InvalidSyntax` with the
detail `expected a decimal without whitespace or separators`.

### Significant-digit scan

The scanner counts precision in the mantissa, before the exponent:

1. Skip the optional leading sign.
2. Visit mantissa bytes until `e` or `E` (or the end for an integer).
3. Ignore every zero before the first nonzero digit.
4. Once a nonzero digit has appeared, count every later mantissa digit,
   including zeroes after the decimal point and trailing zeroes.
5. Exponent digits are not significant digits. If the mantissa contained only
   zeroes, report one digit so the result can retain signed zero or integer
   zero.

The counter uses saturating `u8` arithmetic. The public limits are much lower
than 255, so any count above the limit is rejected; the saturation only bounds
the error's reported `got` value for an extraordinarily long mantissa.

Examples of the scan, independent of payload conversion:

| Input | Significant digits | Reason |
| --- | ---: | --- |
| `0` | 1 | All zeroes are represented by the minimum count. |
| `-0.0` | 1 | The sign and decimal point do not add digits. |
| `000.00120` | 3 | `1`, `2`, and the trailing `0` follow the first nonzero. |
| `001.2300` | 5 | Leading zeroes are ignored; `1`, `2`, `3`, `0`, `0` count. |
| `1e100000` | 1 | Exponent digits never count. The later range check can still fail. |
| `1.00000` | 6 | The first `1` and five following zeroes count. |

`require_digit_bound` runs immediately after this scan. Therefore a value with
too many significant digits returns `TooManySignificantDigits` before f64,
f32, or i32 conversion is attempted. Its detail is exactly
`f32 contract accepts at most 6 significant digits, got N` or
`int32 contract accepts at most 9 significant digits, got N`.

## The f32 contract

`parse_contract_f32` is the normative six-significant-digit representation
boundary. Its result is a finite binary32 payload and the precision proven for
the source decimal.

The implementation order is observable through the error kind and detail:

1. `scan_decimal(input, Float)` validates the float grammar and counts
   significant digits.
2. The six-digit bound is enforced.
3. The original text is parsed as `f64`. A failed conversion returns
   `OutsidePayloadRange` with `decimal cannot be represented by the validation
   domain`. This f64 value is the reference used for the precision comparison.
4. The original text is parsed as `f32`. A failed conversion returns
   `OutsidePayloadRange` with `decimal cannot be represented as f32`.
5. Both parsed values must be finite. A non-finite f64 reference or f32 result
   returns `OutsidePayloadRange` with `calculation payloads require a finite
   f32 representation`. NaN and infinity can therefore never enter a numeric
   column.
6. If the f64 reference is zero, the f32 result must also be zero and its sign
   bit must match. A signed-zero mismatch returns `PrecisionLoss` with
   `signed zero did not survive f32 representation`.
7. If the reference is nonzero but the f32 result is zero, the decimal
   underflowed and the function returns `PrecisionLoss` with
   `nonzero decimal underflows to f32 zero`.
8. The reference and the converted f32 are each formatted by
   `format_at_precision` in scientific notation at the scanned precision.
   Any difference returns `PrecisionLoss` with
   `f32 round-trip is <observed>, expected <expected> at N significant
   digit(s)`.
9. On success, the f32 bits and the nonzero scanned digit count are stored in
   `F32Decimal`.

The comparison is deliberately made through `f64::from(value)` and formatted
at the declared precision. It proves that the binary32 result has the same
decimal value at the precision the source declared. It does not claim that a
decimal with six digits is exactly representable in binary, nor does it retain
the source's textual spelling. Subnormal values go through the same comparison;
an edge whose rounded binary32 value changes the declared digits fails closed
as `PrecisionLoss` instead of weakening the six-digit claim.

Some useful consequences are:

| Input class | Result |
| --- | --- |
| `-0` or `+0` | Accepted with the corresponding zero sign when the f32 parser preserves it. `bits()` distinguishes `-0.0` from `+0.0`. |
| A finite decimal with one through six significant digits that survives the comparison | `Ok(F32Decimal)`. |
| A finite decimal with seven or more significant digits | `TooManySignificantDigits`, even if the f32 value itself could be stored. |
| A nonzero value such as `1e-50` that converts to f32 zero | `PrecisionLoss` for underflow. |
| An exponent or magnitude that f64 or f32 cannot represent, or a result that is infinite | `OutsidePayloadRange`. |
| NaN, infinity, whitespace, or separators | `InvalidSyntax` for lexical spellings such as `NaN` and `inf`, or the corresponding range error if a syntactically valid exponent overflows. |

The table is about the parser's stages, not a promise that every spelling in a
class reaches the same standard-library conversion error. The scanner and
digit bound always run first.

`decimal_round_trip()` uses the same significant-digit count that was checked
by the parser. Consequently, a successful result can be displayed and parsed
again at the proven precision, while a different source spelling with the
same value and a different declared precision remains distinguishable by the
wrapper's `Eq` implementation.

## The int32 contract

`parse_contract_i32` is the exact nine-significant-digit integer boundary:

1. `scan_decimal(input, Integer)` accepts an optional leading sign and ASCII
   digits only. A point or exponent is immediately `InvalidSyntax`.
2. The nine-digit bound is checked before conversion.
3. `input.parse::<i32>()` must succeed. A failed conversion returns
   `OutsidePayloadRange` with `decimal integer is outside the int32 payload
   range`.
4. The exact `i32` and scanned digit count are stored in `I32Decimal`.

The mathematical payload type is signed `i32`, so the standard range is
`-2_147_483_648..=2_147_483_647`. The precision contract is stricter in the
way it is expressed: ordinary decimal spellings of values outside the
nine-significant-digit boundary are rejected as `TooManySignificantDigits`
before the range conversion. For example, `2147483647` and
`-2147483648` each contain ten significant digits and therefore fail the
digit bound rather than reaching the i32 range check. Leading zeroes do not
avoid the bound once the first nonzero digit has appeared.

Conversely, an integer such as `214748364` has nine digits and is within the
payload range. A zero with any number of leading zeroes has one significant
digit. The range error remains part of the public error contract because it is
the direct mapping for a failed `i32` conversion, even though the current
nine-digit bound catches normal out-of-range decimal magnitudes first.

Unlike the f32 parser, the integer parser has no finite check, exponent
handling, signed-zero distinction, or precision round-trip comparison. The
stored `i32` is exact by construction.

## Semantic inference caller

[`ingest/src/semantic.rs`](../../src/semantic.rs) uses the parsers as
lossless evidence, not as an error-producing conversion. `classify_vector`
collects the nonempty cells of one `RawTable` column and tries recognizers in
this order:

1. recognized encoded-image signatures;
2. temporal syntax;
3. a declared ordinal vocabulary;
4. UTF-8 values for which every present cell passes `parse_contract_i32`;
5. UTF-8 values for which every present cell passes `parse_contract_f32`;
6. the caller's ambiguous-vector model for everything else.

The i32 recognizer runs before f32 because the f32 grammar also accepts plain
integer spellings. A column containing only contract-valid integers is thus
`SemanticType::Numeric` with `VectorEncoding::I32`; a column containing a
decimal point or exponent can become `VectorEncoding::F32` when every present
value passes the f32 contract but the i32 recognizer does not. A nonempty cell
that is not UTF-8 or fails one parser makes that recognizer return false for
the whole column. The classifier then tries the next recognizer or delegates
to the ambiguous model; it does not expose the discarded `DecimalError`.

Empty cells are excluded from the `present` iterator. A column with all cells
empty cannot satisfy either numeric recognizer and is delegated to the
ambiguous model. Missingness is retained in the evidence counts and is dealt
with later by preparation. Inference also records the selected encoding's
dtype through `VectorEncoding::dtype`: `I32` maps to `DType::I32`, `F32` maps
to `DType::F32`.

## Prepared table columns

[`ingest/src/prepare.rs`](../../src/prepare.rs) applies an inferred or
caller-supplied semantic contract to a `RawTable` in two phases:

1. `prepare_table` selects columns and rows, computes the exact train
   partition, and runs semantic inference on that train-only view. It then
   applies the fitted schema to every retained row, including validation rows.
2. `prepare_inferred_table` receives an authoritative `InferredVectorList`,
   validates that it describes the table, fits metadata on the train rows, and
   applies the supplied encoding to every retained row without re-inference.

For a numeric schema, `fit_vector_schema` uses `VectorMetadata::None`; numeric
columns have no dictionary, temporal origin, or stored decimal-precision
metadata. `apply_vector_schema` then dispatches as follows:

| Inferred encoding | Nonempty cell | Empty cell | Stored value |
| --- | --- | --- | --- |
| `VectorEncoding::I32` | Convert UTF-8 text with `parse_contract_i32`; a failure aborts preparation. | `None` | `PreparedValues::I32(Vec<Option<i32>>)` |
| `VectorEncoding::F32` | Convert UTF-8 text with `parse_contract_f32`; a failure aborts preparation. | `None` | `PreparedValues::F32Bits(Vec<Option<u32>>)` |

`encode_i32` and `encode_f32` preserve source-row order and zip each cell with
its original source-row index. Every resulting vector has exactly the number
of retained rows. A malformed cell is not silently reclassified during this
phase: invalid UTF-8 or a `DecimalError` is wrapped as
`PrepareErrorKind::EncodingFailure`, with the column bytes and source row
attached. A missing cell is a valid `None` here, so a later operation can
choose whether missing data is legal.

The precision proof is intentionally not copied into `PreparedValues`.
Preparation retains only the calculation payload (`i32` or f32 bits), semantic
encoding, and row alignment. Code that needs the original declared precision
must call the public parser before that information is discarded.

### Dense matrix projection

`PreparedDataset::fixed_dense_matrix` selects one role and one partition and
emits a homogeneous row-major `DenseMatrix` without normalization, feature
derivation, imputation, or lossy casting:

* If every selected vector stores `PreparedValues::I32`, the result is
  `DenseMatrix::I32` and every selected value must be `Some`.
* If any selected vector stores `F32Bits`, the result is `DenseMatrix::F32Bits`.
  Existing f32 bits are copied. An i32 value is converted to f32 only when
  `f64::from(value as f32) == f64::from(value)`; otherwise the operation
  returns `PrepareErrorKind::MixedDenseEncoding` with the column and source
  row. This is the exact-integer-to-binary32 boundary, independent of the
  parser's six-digit decimal proof.
* A missing selected value returns `MissingDenseValue`, a variable-width vector
  returns `VariableWidthDenseMatrix`, an absent role returns
  `EmptyDenseSelection`, and an inconsistent retained position returns
  `InconsistentPreparedVector`.

The dense matrix stores dimensions and values separately. Its `dtype()` is
`DType::I32` for `I32` and `DType::F32` for `F32Bits`; `rows()` and `columns()`
describe the row-major shape. The parser's f32 finiteness guarantee is still
checked by later consumers, so a malformed manually constructed matrix cannot
silently introduce NaN or infinity.

## Row-predicate caller

Preparation can remove source rows with `RowPredicate`. Numeric predicate
handling is deliberately narrower than the column parser:

* A signed or unsigned integer literal requires a fitted
  `SemanticType::Numeric`/`VectorEncoding::I32` column, but the source value is
  compared with `str::parse::<i64>()` or `str::parse::<u64>()`, not with
  `parse_contract_i32`. The predicate therefore has its own integer syntax and
  range behavior.
* A `PredicateLiteral::F32Bits` literal requires a fitted numeric f32 column.
  The literal itself must be finite. Each source value is UTF-8 decoded and
  passed through `parse_contract_f32`, so the six-digit contract applies to
  f32 predicate values. The finite parsed value is then compared with the
  literal's exact bits.
* A text literal uses UTF-8 lexical comparison and is valid only for
  categorical, ordinal, or text semantics.

For `prepare_table` and `select_table`, predicates are resolved before fit, so
an invalid source value is reported as `PrepareErrorKind::PredicateTypeMismatch`
after `predicate_value_error` wraps its UTF-8 or decimal failure. For
`prepare_inferred_table`, the supplied schema is already fitted and the same
failure is `PrepareErrorKind::InvalidPredicateValue`. In all paths an empty
source cell is `MissingPredicateValue`; the row is not treated as a nonmatching
row. Predicate errors carry the predicate column and source row.

## Schema-driven inference caller

[`ingest/src/inference.rs`](../../src/inference.rs) applies a saved
`InferenceFeatureSchema` without semantic inference, dictionary fitting, row
filtering, or train/validation splitting. Feature names are matched by exact
header bytes, and a required name must occur exactly once.

For `InferenceFeatureEncoding::NumericI32`, every source row must be nonempty,
valid UTF-8, and accepted by `parse_contract_i32`; the result is an
`PreparedInferenceValues::I32(Vec<i32>)`. For
`InferenceFeatureEncoding::NumericF32`, the same conditions use
`parse_contract_f32`, and the result is
`PreparedInferenceValues::F32Bits(Vec<u32>)`. Numeric missing cells return
`InferencePrepareErrorKind::MissingValue`; invalid UTF-8, invalid syntax,
range failures, and precision failures return `InvalidValue` with a path that
names the feature, source vector, column, and source row.

Unlike a prepared training table, schema-driven inference has no `Option` in
numeric storage: every row must be present and valid, and each feature's value
length must equal `table.rows().len()`. The `F32Decimal` digit count is again
not retained after the f32 bits are extracted. Categorical dictionaries use a
separate branch and are not parsed as decimals.

## Downstream calculation representation

The parser's output reaches the calculation boundary through fixed-width
values, not through a second numeric parser:

* Dense training lowering consumes `PreparedValues::I32` or `F32Bits`. An i32
  feature or target that must enter an f32 calculation is converted with the
  same exactness comparison used by `fixed_dense_matrix`; a missing value or a
  non-exact conversion is a typed training compile error.
* The training compiler's external-input path serializes `DenseMatrix::I32`
  values with `i32::to_le_bytes` and `DenseMatrix::F32Bits` values with
  `u32::to_le_bytes`. It checks f32 bits for finiteness before admitting them to
  a tensor and checks that byte length equals the declared shape.
* KNN reference preparation follows the same rule: integer references are
  converted to f32 only when exact, f32 references must be finite, and missing
  references are tracked separately from payload zero.

These consumers do not recover the decimal precision count. They consume the
already-proven four-byte payload, preserving the separation between ingestion
proof and GPU calculation.

## Error and invariant matrix

The following matrix distinguishes a `DecimalError` from the outer errors that
carry it:

| Boundary | Empty source cell | Invalid UTF-8 | Decimal parser failure |
| --- | --- | --- | --- |
| Direct `parse_contract_*` call | The empty string is `Empty`; an empty table cell is not a parser call. | Not applicable because the API takes `&str`. | Returns the exact `DecimalError` with input, kind, and detail. |
| Semantic inference | Omitted from numeric recognizer; all-empty vectors go to the ambiguous model. | Numeric recognizer returns false and the model gets the column. | Numeric recognizer returns false and the model gets the column; the error is discarded. |
| Prepared training table | Stored as `None` in the numeric `PreparedValues` variant. | `EncodingFailure` with column and source row. | `EncodingFailure` with the nested `DecimalError` display. |
| F32 row predicate | `MissingPredicateValue`. | `PredicateTypeMismatch` before fit or `InvalidPredicateValue` after fit. | Same outer kind as invalid UTF-8, with the nested decimal detail. |
| Schema-driven inference | `MissingValue`. | `InvalidValue` with `InferenceDataPath`. | `InvalidValue` with `InferenceDataPath` and nested decimal detail. |

The numeric layer itself has no fallback parser, rounding mode option, locale,
separator policy, or host-side normalization path. Once a nonempty value is
admitted, its payload type and bytes are fixed by the selected parser. Once a
value is rejected, callers either report the typed error or, in semantic
inference only, deliberately continue to the next semantic recognizer.

## Practical contract checklist

When adding or reviewing a caller, preserve this sequence:

1. Keep source cells as bytes until the caller has selected a numeric encoding.
2. Treat an empty cell as missing according to the caller's contract; do not
   turn it into the text `0`.
3. Decode UTF-8 explicitly and pass the untrimmed text to the appropriate
   parser.
4. Use `parse_contract_i32` only for an exact integer encoding and
   `parse_contract_f32` only for the finite six-digit f32 encoding.
5. Preserve f32 bits with `bits()` and little-endian serialization; do not
   perform an intermediate host arithmetic round trip.
6. Keep `significant_digits()` only when a caller needs the ingestion proof in
   its own metadata. The standard prepared-column and inference representations
   intentionally retain payload bits, not source precision.
7. Do not replace a parser error with imputation, clipping, rounding, or a
   different numeric type. Those changes would move the representation
   boundary and invalidate the contract described here.
