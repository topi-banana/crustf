# CLAUDE.md

Operational notes for Claude Code (or any agent) working on this repo.

## What this repo is

`crustf` is a Rust workspace that produces JVM class files from Rust code.
The public entry point is the `crustf` umbrella crate; the implementation
lives in:

* `crates/crustf-spec/` — pure data model and binary encoder. No builder
  sugar. Every instruction, attribute, and constant-pool kind in JVMS §4
  is modelled here.
* `crates/crustf-asm/` — fluent builder. Owns label resolution, pool
  interning, auto narrow/wide encoding, `max_locals` inference.
* `crates/crustf-jar-builder/` — ZIP 2.0 writer + `Manifest` + `JarBuilder`.
  Zero external deps, STORED method only. `java -jar` is the integration
  target.
* `crates/crustf/` — re-export umbrella.
* `tests/jvm-integration/` — E2E tests that spawn `java` on emitted bytes.

## Workspace invariants

* Dependency direction is `spec → asm → crustf` only, with
  `crustf-jar-builder` sitting alongside `asm` (no edges from it into the
  bytecode crates — it is a generic archive writer). Do not add an edge
  from `spec` back up to `asm` or from the integration tests to spec
  internals. Tests that need to bypass the builder should reach through
  `crustf::spec::…`.
* `crustf-spec`, `crustf-asm`, and `crustf-jar-builder` must keep
  compiling for `wasm32-unknown-unknown` with `--no-default-features`.
  Every direct `std::*` use needs a `std` feature gate, or should be
  replaced with `alloc::*` / `core::*`.
* `crustf-jar-builder` has zero external dependencies by design. Adding
  a crate for compression or hashing needs explicit discussion because
  it widens the WASM footprint.
* No `unsafe`. The workspace sets `unsafe_code = "forbid"`.

## Default class file version

`ClassFileBuilder::new` defaults to Java SE 5 (major `49`). Class files
with major `≤ 49` are verified with the legacy verifier, so integration
tests with back branches pass without emitting `StackMapTable`. If you
change the default, every test using `ClassFileBuilder::new` without
`.version(...)` will need a `StackMapTable`. Change deliberately.

## Running the full CI locally

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --workspace --all-targets --all-features
cargo test  --workspace --all-features
taplo format --check --diff
cargo machete
cargo build -p crustf-spec        --target wasm32-unknown-unknown --no-default-features
cargo build -p crustf-asm         --target wasm32-unknown-unknown --no-default-features
cargo build -p crustf-jar-builder --target wasm32-unknown-unknown --no-default-features
```

The GitHub workflow in `.github/workflows/ci.yml` runs exactly this set.

## Common pitfalls

* **`StackMapTable`**: if you bump the class file version past `50.0`, the
  HotSpot verifier will reject branches without a matching stack map. The
  builder does not yet synthesise stack maps automatically. Either stay
  on `JAVA_5`, or provide `Attribute::StackMapTable` manually on each
  `Code` attribute.
* **`max_locals`**: `MethodBuilder::into_method` seeds `min_locals` from
  the descriptor (param slots + implicit `this` for non-static methods).
  Users forcing `code.max_locals(n)` override this unconditionally —
  setting a value lower than the real requirement is a verifier error.
* **Modified UTF-8**: all `CONSTANT_Utf8_info` payloads go through
  `crustf_spec::mutf8`. Never feed raw bytes — `NUL` must become `C0 80`
  and supplementary chars must be emitted as surrogate pairs.
* **Long / Double constant pool slots**: `Constant::Long` and
  `Constant::Double` take two logical slots (JVMS §4.4.5). `ConstantPool`
  already places a `None` padding entry automatically via
  `ConstantPool::push`.
* **`tableswitch` / `lookupswitch` padding**: the operands must start on a
  4-byte boundary from the start of the `Code` array. `Instruction::size`
  takes an `offset: usize` argument to compute the pad correctly.

## Adding a new instruction

1. Add the variant to `crustf_spec::instruction::Instruction` and
   `Instruction::opcode` / `Instruction::size` / `Instruction::encode`.
2. If the builder should expose it, plumb through `crustf_asm::code`:
   either add to the `raw_methods!` macro for operand-less opcodes, or
   add a dedicated method that pushes an `Op::Raw(Instruction::…)`.
3. Add a spec-level unit test in `crates/crustf-spec/src/instruction.rs`
   asserting `encode` output bytes.
4. Add an integration test in `tests/jvm-integration/tests/` that
   exercises the instruction against a real JVM if feasible.

## Adding a new attribute

1. Extend `crustf_spec::attribute::Attribute` with the variant plus any
   supporting structs. Keep the variant body in JVMS order.
2. Add an entry to `Attribute::name()` returning the JVMS attribute name.
3. Handle the variant in `crustf_spec::encode::encode_attribute_body`.
4. In `crustf_asm::builder`/`method`/`field`, intern the name before
   pushing the attribute so the encoder can resolve the `name_index`.
5. Add a unit test emitting the attribute and asserting on bytes, plus an
   integration test if the attribute is observable from Java runtime.

## Commit hygiene

* Small, focused commits. Tests with the feature they cover.
* Commit messages follow imperative mood ("Add lookupswitch builder
  helper", not "Added ..." / "Adds ...").
* Never commit with `--no-verify`.
