# Soul Compiler — Roadmap / TODO

Source of truth for design decisions lives in [`docs/`](docs/):
[compiler-pipeline-plan.md](docs/compiler-pipeline-plan.md),
[mir-design.md](docs/mir-design.md),
[crate-system-plan.md](docs/crate-system-plan.md),
[reflection-system.md](docs/reflection-system.md),
[ANCHORED-crate.md](docs/ANCHORED-crate.md).
This file just tracks status and next steps — update it as work lands, don't duplicate design
rationale from those docs here.

## Pipeline

```
lexer → parser → AST → name/typecheck → MIR → LLVM IR (inkwell) → .exe
```

## Milestones

### M1 — first real exe (in progress)

Concrete, non-generic subset: ints, arithmetic, `println`, functions, structs, non-generic traits.
No `Res`/`.pass`/`?T`, no unions, no generics, no borrow checking yet.

- [x] Tokenizer, parser, name resolver (pre-existing, ahead of the rest of the pipeline)
- [x] MIR shapes defined (`soul_mir/mir_model`) — see [mir-design.md](docs/mir-design.md)
- [x] MIR lowering: assignments, primitive checks (`mir_run` / lowering, per recent commits)
- [x] MIR lowering: if/for/break/continue
- [x] MIR lowering: comparisons and logical ops
- [x] MIR lowering: none-returning functions and calls
- [x] Bare assert/panic and undefined-fn checks
- [x] MIR display/serialization, fault plumbing
- [x] Struct name resolution: `DeclareStore.struct_names: VecMap<ModuleId, HashMap<SharedStr, NodeId>>`
      (`get_struct_by_name`), populated in `try_insert_struct`, lets a later pass resolve a
      struct-typed `SoulType::Stub`'s bare name back to its `Struct` declaration without re-walking
      scopes — keyed by `ModuleId` first (rather than a flat `HashMap<(SharedStr, ModuleId), _>`) so a
      lookup hashes a bare `&str` instead of allocating an owned `SharedStr`. Turned out narrower than
      first scoped during grill-me ("build a whole declare+resolve system mirroring functions") —
      `soul_name_resolver` already had working struct field-type-checking via
      `lookup_type`/`get_custom_type` (proved by the passing `field_access_type_tests.rs`); the only
      real gap was this one name index, not missing anywhere.
- [x] Struct field reads/writes/construction lowered through the whole pipeline and proven via real
      exes (`09_struct_field_read.soul`, `10_struct_field_write.soul`, per the "prove one vertical
      slice before extending further" decision from grill-me):
  - `mir_parser`: `FunctionLowerer` resolves each function's module once via `declares.get_function`,
    accepts struct-typed params/locals/returns (`require_lowerable`), lowers `Struct{..}` construction
    to `Rvalue::Aggregate(AggregateKind::Struct, ..)` with operands reordered to the struct's
    *declared* field order (not literal order), and lowers `variable.field` reads/writes to a `Place`
    with a `PlaceElem::Field(index)` projection reusing the variable's own storage (no copy) — shared
    via `resolve_field_place`, used from both `lower_operand` (read) and `lower_assignment` (write);
    mutability isn't enforced here (that's the M2 borrow checker's job)
  - `mir_codegen`: `llvm_type` builds an LLVM struct type per resolved `Stub`; `resolve_place` walks
    an arbitrary-length `Field` projection chain via `build_struct_gep` for both loads and stores (no
    codegen changes were needed to support nested chains — it was already general); `Rvalue::Aggregate`
    codegens via `get_undef`+`build_insert_value` per field
  - [x] Nested field chains (`o.inner.x`, both reads and writes): `resolve_field_place` in `mir_parser`
    now recurses when the field-access object is itself a field access, building one `Place` with a
    multi-element `Field` projection rather than a chain of temporaries
  - Not yet supported: struct-typed binary-op operands' signedness (`operand_is_signed` doesn't look
    through a `Field` projection — doesn't matter for a bare field read, would matter for `p.x - 1`
    on a signed field)
- [x] Array literal construction, `&arr`-to-slice, and slice indexing (read+write), plus bounds
      checking on slice indexing — proven via `12_slice_index.soul` and `13_slice_bounds_check.soul`.
      Scoped to fixed-size arrays (`[N]T`, only as the thing you *reference*) and slices
      (`[&]T`/`[&mut]T`, only as the thing you *index*) — not wildcard-sized (`[_]T`) or heap (`[]T`)
      arrays:
  - `mir_parser`: generalized `resolve_field_place` into `resolve_place_expr`, a shared place
    resolver dispatching on variable/field-access/index (so `o.items[i].x` composes into one `Place`
    with a three-element projection, same pattern as nested field chains). `[1, 2]` lowers to
    `Rvalue::Aggregate(AggregateKind::Array, ..)` in literal order (arity trusted from the resolver,
    same as struct constructors). `&arr` on a fixed-size-array place lowers to *two* statements — a
    plain `Rvalue::Ref` into a pointer temp, then `Aggregate(Array, [ptr, compile-time-constant len])`
    — deliberately not a single-step `Rvalue::Ref`, so `Ref` itself stays bare-pointer-only per the
    original grill-me decision. `collection[index]` lowers to a `Place` with a `PlaceElem::Index`
    projection; the index expression is always materialized into a `uint` temp (`operand_local`) to
    avoid width-mismatch risk from re-typing whatever concrete int type it already had.
  - `mir_codegen`: `llvm_type` maps `[N]T` to a real LLVM array type and `[&]T`/`[&mut]T` to a
    `{ptr, len}` struct (`len` at pointer width). `resolve_place`'s walk now tracks the *Soul* type
    (not just the LLVM type) through each projection step — unlike a struct field (queryable straight
    off its LLVM `StructType`), an opaque LLVM pointer carries no pointee-type info at all, so the
    element type after an `Index` step has to come from the `ArrayType` on the Soul side instead.
    `Rvalue::Ref` codegens for the first time (just the address `resolve_place` computes, no load —
    by construction it's never handed an array-typed place, see above). `codegen_aggregate` (renamed
    from the struct-only version) now branches on `StructType` vs `ArrayType` destinations, since a
    fixed-size array's `insertvalue` target isn't struct-shaped.
  - Bounds checking and overflow checking were originally implemented as codegen-level ad hoc
    branch-splitting (see history), then **moved into MIR itself**, mirroring rustc's own `Len`/
    `CheckedBinaryOp`/`Assert` shapes — see the two `[x]` entries directly below. `mir_codegen` no
    longer decides *when* to trap at all; it just implements the (now fully generic) MIR primitives
    that make the trap, and every check is visible in the MIR dump instead of only in the LLVM IR.
- [x] Bounds checking on slice indexing, **at the MIR level** (moved off the original codegen-level
      implementation — see history at the bottom of this file for that version) — proven via
      `13_slice_bounds_check.soul`.
  - `mir_model`: new `Rvalue::Len(Place)` (the slice's own runtime `len`, mirroring rustc's `Len`).
  - `mir_parser`: `resolve_index_place`'s `emit_bounds_check` now emits, ahead of the actual
    `PlaceElem::Index` projection: `len_local = Len(collection)`; casts the index to `uint` first if
    it isn't already one (`operand_local` reuses a bare-`Variable` index's own declared type as-is,
    so it isn't always pre-normalized — see the new `Rvalue::Cast` below); `cond = index < len`; then
    seals the block with `Terminator::Assert { cond, expected: true, msg: "index out of bounds",
    target: next }`. Bounds checking is now indistinguishable, MIR-shape-wise, from a hand-written
    `assert(i < s.len())` — `mir_codegen` doesn't know or care that it came from indexing.
  - `mir_codegen`: `Rvalue::Len` loads field `1` of the slice's `{ptr, len}` fat pointer
    (`rvalue::codegen_len`). `step_into_index` no longer does any bounds checking at all — it trusts
    the index is in range, exactly as if the MIR simply never proved otherwise.
  - Not yet supported: indexing a raw `[N]T` directly (only a slice can be indexed — reference it
    first), `&`/`@` mutability not checked against `[&]`/`[&mut]` (that's the M2 borrow checker's job,
    same as struct field mutability)
- [x] Overflow checking on arithmetic ops (`+`/`-`/`*`), **at the MIR level** (moved off the original
      codegen-level implementation) — proven via `14_arith_overflow_check.soul` (`i32::MAX + 1`
      aborts instead of wrapping).
  - `mir_model`: new `Rvalue::CheckedBinaryOp(op, left, right)`, producing a `(T, bool)` tuple
    (result, overflowed) — mirrors rustc's own `CheckedBinaryOp` shape exactly. Reuses
    `AggregateKind`'s pre-existing (until now unused) `Tuple` variant's *type* side —
    `SoulType::TupleKind(TupleKind::Tuple(..))` — not a new `Rvalue` aggregate-construction case
    (the tuple value itself is never built via `Aggregate`; the intrinsic call's own return value
    *is* the tuple, see below).
  - `mir_parser`: `is_checked_arith_op` routes `Add`/`Sub`/`Mul` (only) through
    `lower_checked_binary_op` instead of a plain `Rvalue::BinaryOp` — assigns `CheckedBinaryOp` into
    a fresh tuple-typed temp, seals the block with `Terminator::Assert { cond: tuple.1, expected:
    false, msg: "attempt to {add,subtract,multiply} with overflow", target: next }`, then returns
    `Rvalue::Use(Copy(tuple.0))` as if this had been an ordinary `BinaryOp` all along — every
    existing call site (`lower_operand`'s nested-expression materialization, a top-level
    `lower_assignment`/`lower_variable`/`return`) needed no change at all. The tuple's element type
    is derived from whichever *operand* actually carries a place-backed type (a new `operand_type`/
    `place_type` pair, mirroring `mir_codegen::rvalue`'s own `operand_type` one layer up) rather than
    the resolver's per-expression type table — needed because the resolver never types a
    `FieldAccess`/`Index` *expression* (`s[0] + s[1]`'s own type would otherwise be unresolvable),
    falling back to the resolver's whole-expression type only when both operands are bare constants
    (`3 + 4`, neither a place).
  - `mir_codegen`: `Rvalue::CheckedBinaryOp` calls the matching LLVM `{s,u}{add,sub,mul}.with.overflow`
    intrinsic and returns its `{result, i1 overflowed}` struct *directly* as the tuple value — LLVM
    uniques anonymous struct types structurally, so the intrinsic's own return type and the
    synthesized tuple `StructType` are the same type, no repacking needed. Deciding whether to trap
    isn't `mir_codegen`'s job any more; it only computes the pair.
  - New `Rvalue::Cast(Operand, Type)` codegen (previously declared in `mir_model` but never
    implemented) — sign-extends/zero-extends/truncates an int operand to the destination width,
    based on the *source*'s signedness. Currently only reachable from the bounds-check index
    normalization above; general implicit/explicit int-to-int casts elsewhere in the language aren't
    wired to it yet.
  - `step_into_field` (struct-field `Place` resolution) now also accepts a positional-tuple
    `SoulType::TupleKind(TupleKind::Tuple(..))` destination alongside a declared struct — needed so
    `PlaceElem::Field(0)`/`Field(1)` can address a `CheckedBinaryOp` tuple's result/overflow halves,
    not just a real struct's fields; `llvm_type` gained a matching `TupleKind::Tuple` → anonymous
    LLVM `StructType` mapping.
  - Div/Mod were untouched by this pass — division overflow (`INT_MIN / -1`) and div-by-zero are
    handled separately below.
- [x] Overflow checking on `/`/`%` (division by zero, and — signed only — `MIN / -1`/`MIN % -1`) —
      proven via `17_div_by_zero_check.soul`, `18_mod_by_zero_check.soul`, `19_div_overflow_check.soul`,
      `21_mod_overflow_check.soul` (all abort with the matching message+location), plus
      `20_div_mod_normal.soul` as a regression guard that ordinary division/remainder still compute
      the right answer with checking on.
  - Unlike `+`/`-`/`*`, there's no `{s,u}div/rem.with.overflow` LLVM intrinsic to call, so this
    doesn't produce a `(T, bool)` tuple the way `CheckedBinaryOp` does — `mir_parser`'s new
    `lower_checked_div` instead emits explicit MIR-level `Assert`s ahead of an ordinary, now-safe
    `Rvalue::BinaryOp(Div/Mod, ..)`, mirroring rustc's own checked-division lowering (which does the
    same thing for the same reason): `assert(divisor != 0, "attempt to {divide,calculate the
    remainder with a divisor of} ... zero")`, then — only when the operand type is signed —
    `assert(!(dividend == MIN && divisor == -1), "attempt to {divide,calculate the remainder} with
    overflow")`. `mir_codegen` needed zero changes: by the time `Div`/`Mod` reach
    `codegen_binary_op`, the checks already ran, so the existing `build_int_signed_div`/`_rem`/
    `_unsigned_div`/`_rem` arms are unchanged.
  - Computing the actual `MIN` constant needed the operand's concrete bit width, which — for
    `int`/`cint` — is platform-sized (`PlatformInfo.pointer_bits`/`c_int_bits`) and only known once
    `mir_parser` is handed a `&CompilerOptions` (already true, from the `MirOptions` toggle work).
    New `signed_primitive_min(prim, platform)` is the one place in `mir_parser` that reads
    `PlatformInfo` for this reason, despite that struct's own doc comment saying nothing upstream of
    codegen normally needs to — there's no way to express "the minimum value of whatever width this
    turns out to be" as a single width-agnostic MIR constant the way `0`/`-1` already are.
  - `mir_model::Operand` gained `Clone` (needed to reuse the same dividend/divisor operand across
    both the division itself and its guard-condition comparisons).
  - Gated behind the same `MirOptions::CHECK_ALGORITHMIC_OVERFLOW` flag as `+`/`-`/`*` rather than a
    separate flag — division-by-zero and overflow are bucketed with the rest of "checked arithmetic"
    here, not split out on their own.
- [x] Rust-`panic!`-style panic runtime (message, no backtrace, no unwinding) — every panicking
      construct is now an ordinary MIR `Terminator::Assert` (bounds check, overflow check,
      `assert(cond)`/`panic(msg)` — see the bounds-checking/overflow-checking entries above for how
      the first two now get there), and `codegen_assert` is the *only* place `mir_codegen` ever calls
      the panic runtime from. Proven via `15_assert_panic_message.soul` and
      `16_panic_intrinsic_message.soul` (stdout matched against `panic: <message>`), plus
      `expect_stdout` added to `13_slice_bounds_check.soul`/`14_arith_overflow_check.soul`.
  - `mir_codegen/src/terminator.rs`: `panic_function` lazily declares *and defines* (once per module)
      a `soul_panic(msg: cstr)` function — `printf("panic: %s\n", msg)`, `fflush(NULL)`, `abort()`,
      `unreachable` — building its body with the same per-function `self.builder` used for the
      function currently being codegen'd (saves/restores the builder's insertion point around it,
      since there's no separate builder per LLVM function). `codegen_assert` passes `Assert`'s own
      `msg` operand straight through to it (already codegen'able via the existing `cstr` operand
      path) — no bespoke codegen-level "materialize a message, split a block, trap" helper exists any
      more (an earlier version of this pass had `trap_if`/`trap_with_message` for that; both were
      deleted once bounds/overflow checking moved into MIR, since every trap now just *is* an
      `Assert`).
  - Real bug found and fixed along the way: `printf`'s output sat in a fully-buffered `stdout` and was
      silently lost, since `abort()` terminates the process immediately without libc's normal at-exit
      flush — caught by actually running a built exe and checking its stdout, not just its exit code
      (an `expect_stdout` match would've silently short-circuited to "test never printed anything" had
      it not been checked by hand first). Fixed with an `fflush(NULL)` call between `printf` and
      `abort()`.
  - Also found and fixed: `codegen_assert` indexed `self.blocks[*target]` unconditionally, but an
      unconditional `panic(msg)` lowers to an `Assert` whose `target` is never given a real block
      (`lower_panic_intrinsic`'s own docs say so — the "ok" path is provably unreachable) — this
      panicked on the very first exe test that actually exercised `panic(msg)` end-to-end. Fixed by
      branching straight to the panic block when `self.blocks.get(*target)` is `None`, instead of
      indexing.
  - [x] Panic location (`file:line:col`, à la `thread 'main' panicked at src/main.rs:4:5`) — proven
      via `expect_stdout` assertions on all four panic-message exe tests (`13`-`16`) matching
      `"<file>.soul:<line>:"`. Still no backtrace (explicitly out of scope, per the user's own
      framing).
    - `mir_model::Terminator::Assert` gained a `span: Span` field — the only MIR shape change needed;
      `mir_parser` already had the span in hand at every `Assert`-emission site
      (`emit_bounds_check`, `lower_checked_binary_op`, `lower_assert_intrinsic`,
      `lower_panic_intrinsic`) and just had to stop discarding it.
    - `mir_codegen` needed a way to turn a `Span`'s `ModuleId` into an actual file path, which nothing
      in `mir_codegen`/`mir_parser` carries today (only the top-level driver's own
      `soul_utils::collections::module_store::ModuleStore` does — the same one
      `soul_tester::display::fault` already uses for its own compile-time diagnostics). Threaded a
      `&ModuleStore` through `to_llvm` → `codegen_module` → `CodegenCtx` (a new field alongside
      `declares`/`platform`) so `terminator::codegen_assert`'s new `location_string` helper can
      resolve it and format `"{path}:{line}:{col}"` (the `Span`'s *start* position only — a single
      point, like Rust's own panic locations, not the `start..end` range `Span`'s `Debug` impl prints
      for compile-time diagnostics) as its own global string constant, reusing
      `codegen_string_constant` (back to `pub(crate)`, shared with `rvalue.rs`).
    - `soul_panic`'s signature widened to `(msg: cstr, location: cstr)`, printing
      `"panic: {msg}\n  at {location}\n"`.
    - `scripts/run_codegen_tests.py`'s `// expect_stdout:` was single-shot (first match only, later
      ones silently ignored) — generalized to collect every `expect_stdout` line in a file so a test
      can assert both the message and the location independently.
- [ ] Finish MIR lowering coverage for M1 language surface (non-generic traits) — div-by-zero is now
      handled (see the `/`/`%` overflow-checking entry above), so this is just traits now
- [x] `soul_mir/mir_codegen` — LLVM IR emission via `inkwell` (`features = ["llvm16-0"]`, Windows
      only) implemented for scalar/pointer/struct locals, arithmetic/comparison/logical ops, if/while,
      function calls, `extern "C"` functions (incl. `cstr`/pointer params and correct C-vs-Soul
      integer widths via `PlatformInfo`, see below), string/cstr constants, and process-exit-code
      `main` codegen. Structured fault system (`CodegenErrorKind`) replaces `anyhow`.
  - [x] `f32`/`f64` arithmetic (`+`/`-`/`*`/`/`/`%`) and comparisons (`==`/`!=`/`<`/`>`/`<=`/`>=`) —
        proven via `22_float_arithmetic.soul`, `23_float_ops.soul` (all six ops + all six
        comparisons on `f64`), `24_f32_arithmetic.soul`. `f16` deliberately still unsupported (falls
        through to `UnsupportedPrimitiveType` in `llvm_type` — no direct C ABI use for it yet).
    - `ast_model`/tokenizer/`soul_name_resolver` already had everything needed (`PrimitiveTypes::
      Float16/32/64/UntypedFloat`, `Literal::Float(f64)`, float-literal lexing, and the resolver's
      numeric-promotion tables) — this was purely a `mir_codegen`/`mir_parser` gap, not a frontend one.
    - `mir_codegen::types`: `llvm_type` maps `Float32`/`Float64` (and `UntypedFloat`, defaulting to
      `f64` the same way an untyped int literal defaults to `int`) to the matching LLVM `FloatType`.
      New `const_float`/`expect_float` mirror `const_int`/`expect_int`.
    - `mir_codegen::rvalue`: `codegen_binary` branches to a new `codegen_float_binary_op` when the
      resolved operand type is a `FloatType` — no signed/unsigned split (floats have none), using
      `build_float_add/sub/mul/div/rem` and `build_float_compare` with the *ordered* (`O*`)
      `FloatPredicate`s (`NaN` compares false against everything per IEEE 754, matching how every
      other language defines float `==`/`<`/etc.). `codegen_constant` gained a `FloatType` arm.
    - `mir_parser`: floats are explicitly kept **out** of the checked-arithmetic/checked-div lowering
      (`is_float_operand`, checked before routing into `lower_checked_binary_op`/`lower_checked_div`)
      — IEEE 754 overflow saturates to `inf`/`-inf` rather than being UB the way integer overflow and
      `INT_MIN / -1` are, and there's no `{s,u}*.with.overflow`-style intrinsic for floats anyway, so
      a float `+`/`-`/`*`/`/`/`%` always lowers to a plain, unchecked `Rvalue::BinaryOp`.
    - Not yet supported: `f16`, int↔float casts (`Rvalue::Cast` codegen still assumes an `IntType`
      destination), float constants/locals inside structs or arrays (structurally should already work
      once `llvm_type` recurses into a float field/element, but untested).
- [x] Wire codegen output through to an actual `.exe` — `scripts/run_codegen_tests.py` drives
      `clang.exe` (`C:\llvm-16\bin\clang.exe`) over the emitted `.ll`, then runs the resulting exe
      and checks both exit code (`// expect: N`) and stdout (`// expect_stdout: <substring>`); this
      is currently the real correctness oracle for the pipeline (`soul_tester/soul/src/codegen_tests/`,
      24 passing exe tests). Still manual/script-driven, not integrated into `cargo test`.
- [ ] Establish positive+negative test pairs as typecheck/MIR lowering lands (currently unclear
      whether existing parser/resolver suites cover rejection cases — see "Testing strategy" in
      compiler-pipeline-plan.md)
- [ ] `PlatformInfo` (`soul_utils::compiler_options`) only has one constructor
      (`new_windows_x86_64`, 64-bit pointers / 32-bit C `int`) — no actual cross-platform selection
      logic exists yet, it's just a named default

### M2 — borrow/move checking (not started)

- [ ] Implement borrow checker as a MIR pass over the concrete (M1) subset
- [ ] Verify `Drop`/`MarkMoved`/`SetDropFlag` semantics from mir-design.md are fully emitted by
      lowering before the checker can consume them

### M3 — unions, generics, full traits (not started)

- [ ] Monomorphization (expansion to concrete types before LLVM)
- [ ] Full trait resolution incl. multi-impl-by-output-type-generic case (§7)
- [ ] `Res` / `.pass` / `?T` / match-chains
- [ ] Extend borrow checker to generic MIR with `AutoCopy` bounds
- [ ] `limit N` on `for` loops (blocked on `Res` error shape)
- [ ] `SwitchInt` discriminant handling for union tags (`Rvalue::Discriminant`)

### M4 — everything else (not sequenced)

- [ ] `async`/`await` + structured concurrency runtime
- [ ] Const generics (`Limit<T, RANGE>`)
- [ ] Full reflection (`intrinsic.typeinfo`) — AST/compile-time design exists in
      [reflection-system.md](docs/reflection-system.md), backend (HIR/MIR/LLVM) not started
- [ ] Operator overloading
- [ ] Goul

## Crate system (parallel track)

Tracked in detail in [crate-system-plan.md](docs/crate-system-plan.md) and
[ANCHORED-crate.md](docs/ANCHORED-crate.md). `CrateForest` is wired through parser/name-resolver
and the build is green (143 tests passing as of last update there).

- [x] `CrateId` + `Linkage` in `soul_utils`
- [x] `CrateForest` replacing flat `AstModuleStore` in `AstTree`
- [x] Parser routes modules via `forest`
- [x] Name resolver routes via `forest`
- [x] `ast_run` passes forest through
- [ ] `soul_tester` orchestration: dependency compilation, timestamp-based freshness check,
      `.soulo` rebuild
- [ ] `.soulo` prebuilt artifact: write/read the `SOULO` header + object bitcode format
- [ ] Wire `CrateForest.external` into name resolver for cross-crate name resolution
- [ ] Codegen separation using `ExternalCrateData` (static vs dynamic linkage)
</content>
