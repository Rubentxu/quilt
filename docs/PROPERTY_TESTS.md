# Property-Based Testing

We use [proptest](https://github.com/proptest-rs/proptest) for property-based testing.

## What is property-based testing?

Instead of writing tests with specific inputs and expected outputs, you write
**properties** — invariants that should hold for ALL inputs. Proptest generates
hundreds of random inputs (default: 256 cases per test) and tries to find a
counter-example. If it does, it **shrinks** the input to the minimal failing
case, making the bug easy to reproduce.

Property-based testing is especially powerful for:

- **Parsers** — any string should parse without panicking
- **Sanitizers** — any adversarial input should produce safe output
- **Pure functions** — `f(x) == f(x)` (determinism), `f(g(x)) == f(g(x))` (idempotence)
- **Orderings** — `a < b`, `b < c` ⟹ `a < c` (transitivity)
- **Numeric invariants** — `a + (-a) == 0`, `a - b == -(b - a)`

## Running

Property tests live alongside regular unit/integration tests. They run with:

```bash
cargo test --workspace
```

You can target a specific proptest file:

```bash
cargo test -p quilt-core --test parser_proptest
```

To run a specific property:

```bash
cargo test -p quilt-core --test parser_proptest parser_never_panics
```

### Proptest regressions

When a property fails, proptest writes a `.proptest-regressions` file next
to the test file with a seed of the failing input. On subsequent runs,
that seed is re-played FIRST so the bug stays fixed. These files are
checked into the repo on purpose.

## Configuration

The default 256 cases is usually enough. To increase the case count
(useful for stress-testing critical functions):

```rust
proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn my_test(s in ".*") {
        // ...
    }
}
```

## Invariants tested

| Crate           | File                                       | Properties                                                                                        |
|-----------------|--------------------------------------------|---------------------------------------------------------------------------------------------------|
| `quilt-core`    | `tests/parser_proptest.rs`                 | Parser never panics; segments within bounds; non-overlapping; bold/page-ref recognized; pure; unicode-safe |
| `quilt-search`  | `tests/sanitize_proptest.rs`               | Balanced quotes; tokens are quoted; empty handling; prefix preserved; operators stripped; token count bounded; no crash on adversarial; unicode preserved |
| `quilt-domain`  | `tests/order_proptest.rs`                  | Reindex count/endpoints/spacing; insert-after midpoint between siblings; insert-first below min; robust to permutation |
| `quilt-domain`  | `tests/journal_day_proptest.rs`            | String roundtrip; whitespace trim; invalid month/day rejected; ordering consistent; add-zero identity; add+sub identity; sub anti-symmetric; sub == days_between |

**Total: 37 property-based tests** (9 + 8 + 10 + 10).

## Bugs found by proptest

Proptest is most valuable when it surfaces bugs hand-written tests miss.
Adding these tests surfaced the following real issues — all fixed in
the same change.

### 1. `InlineParser` panics on multi-byte UTF-8 (plain text branch)

**Symptom**: `parse("🉠")` panicked with
`start byte index 1 is not a char boundary; it is inside '🉠' (bytes 0..4)`.

**Root cause**: the plain-text fallback loop in
`crates/quilt-core/src/parser/inline.rs` incremented the byte position
by 1 each iteration, landing in the middle of multi-byte UTF-8 chars.
The next `&content[end..]` slice then panicked.

**Fix**: advance by `c.len_utf8()` instead of `1`.

### 2. `InlineParser::try_parse_property` panics on multi-byte whitespace

**Symptom**: `parse("\u{85}*")` panicked with
`start byte index 1 is not a char boundary; it is inside '\u{85}' (bytes 0..2)`.

**Root cause**: in the property parser, the word-start was computed as
`rfind(is_whitespace).map(|i| i + 1)`. For multi-byte whitespace
chars (e.g. `U+0085 NEXT LINE`, 2 bytes), `+1` lands inside the char,
and the subsequent `&content[word_start..]` slice panics.

**Fix**: use `char_indices().filter(is_whitespace).last()` and advance
by `c.len_utf8()`.

## When to add a property test

- The function takes a `&str`, `&[T]`, or other broad input type.
- The function is pure (no I/O, no global state).
- The contract is "for all valid X, some invariant holds".

If a function is "do this exact thing for this exact input", a
hand-written example test is still better. Properties are for invariants.
