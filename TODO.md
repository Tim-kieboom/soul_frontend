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
- [ ] Finish MIR lowering coverage for M1 language surface (structs, non-generic traits,
      diverging calls for div-by-zero / out-of-bounds per mir-design.md)
- [ ] `soul_mir/mir_codegen` — currently an empty stub (`lib.rs` only); implement LLVM IR emission
      via `inkwell` (`features = ["llvm16-0"]`, Windows only)
- [ ] Wire codegen output through to an actual `.exe` (manual `llc`/linker step for now, per
      compiler-pipeline-plan.md)
- [ ] Establish positive+negative test pairs as typecheck/MIR lowering lands (currently unclear
      whether existing parser/resolver suites cover rejection cases — see "Testing strategy" in
      compiler-pipeline-plan.md)

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

## Housekeeping / uncommitted state

- `soul_mir/mir_codegen/` and `soul_tester/soul/src/codegen_tests/` exist on disk but are new/
  untracked — fold into the M1 codegen work above once they have real content.
- `scripts/` is untracked — check in or .gitignore depending on intent.
</content>
