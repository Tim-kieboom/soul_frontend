# Rust Project Structure

This repository is organized as a set of Rust crates that form a compiler pipeline:

`source -> tokenizer -> AST -> name resolution -> HIR -> typed HIR -> MIR -> LLVM IR`

## Top-Level Layout

```text
soul_frontend/
├─ soul_tokenizer/
├─ soul_ast/
│  ├─ ast_model/
│  ├─ ast_parser/
│  ├─ soul_name_resolver/
│  └─ run_ast/
├─ soul_hir/
│  ├─ hir_model/
│  ├─ hir_parser/
│  ├─ typed_hir_model/
│  ├─ typed_hir_parser/
│  ├─ hir_literal_interpreter/
│  └─ run_hir/
├─ soul_mir/
│  ├─ mir_parser/
│  └─ run_mir/
├─ soul_ir/
├─ soul_utils/
└─ soul_tester/
```

## What Each Rust Project Does

### `soul_utils` (`soul_utils`)
- Shared foundation crate used by almost every other crate.
- Provides common types and helpers: IDs, spans, error/fault types, compile options, maps/sets, symbols, and utility macros.

### `soul_tokenizer` (`soul_tokenizer`)
- Lexical analysis layer.
- Converts source text into a `TokenStream` (`to_token_stream`), which is consumed by the parser.

### `soul_ast/ast_model` (`ast`)
- AST data model crate.
- Defines syntax tree structures (`AbstractSyntaxTree`, declarations, scopes, metadata, display helpers).

### `soul_ast/ast_parser` (`ast_parser`)
- Parser crate for the language syntax.
- Consumes `TokenStream` and builds an `AstResponse` (tree + declaration store + metadata).

### `soul_ast/soul_name_resolver` (`soul_name_resolver`)
- Semantic name-resolution pass over AST.
- Collects declarations, resolves identifiers, and records semantic faults.

### `soul_ast/run_ast` (`run_ast`)
- Orchestrator for the AST phase.
- Runs parsing + name-resolution in one step (`to_ast`).

### `soul_hir/hir_model` (`hir`)
- High-level IR model crate.
- Defines HIR node/types/maps and the core `HirTree` structure.

### `soul_hir/hir_parser` (`hir_parser`)
- AST -> HIR lowering pass.
- Translates resolved AST into HIR while creating IDs, spans, and semantic mappings.

### `soul_hir/typed_hir_model` (`typed_hir`)
- Typed-HIR data structures.
- Holds normalized type graph/map (`ThirTypesMap`) and per-node type table (`TypeTable`).

### `soul_hir/typed_hir_parser` (`typed_hir_parser`)
- Type inference/checking pass for HIR.
- Produces `TypedHir` from `HirTree`, including inferred/known types for expressions, statements, locals, and functions.

### `soul_hir/hir_literal_interpreter` (`hir_literal_interpreter`)
- Compile-time literal evaluator.
- Resolves HIR literals/constant expressions into a normalized literal representation.

### `soul_hir/run_hir` (`run_hir`)
- Orchestrator for HIR phases.
- Runs: HIR lowering -> typed-HIR generation -> literal resolution, and returns `HirResponse`.

### `soul_mir/mir_parser` (`mir_parser`)
- HIR -> MIR lowering pass.
- Builds machine-oriented MIR control flow and statements from typed HIR data.

### `soul_mir/run_mir` (`run_mir`)
- MIR orchestration wrapper.
- Runs MIR lowering and returns `MirResponse`.

### `soul_ir` (`soul_ir`)
- LLVM IR backend.
- Converts MIR + type information into LLVM IR (`to_llvm_ir`) using `inkwell`.

### `soul_tester` (`soul_tester`)
- Executable/test harness crate (`main.rs`).
- Runs the full frontend/backend pipeline, logs faults, and writes artifacts (token/AST/HIR/MIR/LLVM outputs).

## Pipeline Ownership by Crate

- **Lexing:** `soul_tokenizer`
- **Parsing + AST:** `ast_model`, `ast_parser`, `run_ast`
- **Name resolution:** `soul_name_resolver`
- **HIR lowering:** `hir_model`, `hir_parser`
- **Type system + inference:** `typed_hir_model`, `typed_hir_parser`
- **Const/literal interpretation:** `hir_literal_interpreter`
- **MIR lowering:** `mir_parser`, `run_mir`
- **LLVM emission:** `soul_ir`
- **End-to-end runner:** `soul_tester`
