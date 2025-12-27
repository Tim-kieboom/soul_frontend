## Frontend / semantic
(D) Extend Name resolver ot:
    - Duplicate declarations in same scope 
(A) Extend type resolver to:
    - Check binary/unary operator type rules 
    - Check assignment compatibility  
    - Check function call arity and argument types 
(B) Add control-flow-related checks:
    - `return` type vs function return type 
    - `break` / `continue` only inside loops 
    - All paths return in non-void functions (if language requires) 
    - Use-before-declaration 

## Internal HIR (own IR before LLVM)

(G) Design a simple, typed IR for your language (expression + statements + basic blocks)
(H) Implement lowering: typed AST → internal IR
(I) Implement IR validation (sanity checks)
(J) Add an IR pretty-printer / dumper for debugging

## LLVM integration

(K) Choose Rust LLVM binding:
    - `llvm-sys` (raw bindings) or
    - Higher-level wrapper like `inkwell`
(L) Initialize LLVM context/module/IRBuilder for your compiler
(M) Map language types → LLVM types:
    - Integers, booleans, pointers
    - Functions and function pointers
(N) Map internal IR → LLVM IR:
    - Functions (prototypes + bodies)
    - Local variables and allocations
    - Expressions (arithmetic, comparison, boolean)
    - Control flow (if, loops, returns) via basic blocks
(O) Add option to dump LLVM IR to `.ll` files for debugging

## LLVM optimization & codegen

(P) Create and configure an LLVM pass manager (basic optimizations):
    - Constant folding / simple algebraic simplifications
    - Dead code elimination
    - CFG simplification
(Q) Configure target:
    - Target triple
    - Data layout
(R) Emit:
    - Object file (`.o`) or executable via platform linker

## Diagnostics & tooling

(S) Ensure all AST nodes carry source spans
(T) Ensure all `SemanticFault`s carry:
    - Span / node_id
    - Error code / kind
    - User-facing message and hint
(U) Add CLI flags:
    - `--ast` (dump AST)
    - `--ir` (dump internal IR)
    - `--llvm-ir` (dump LLVM IR)
    - `--check` (stop after semantic analysis)

## Testing

(V) Small sample programs for:
    - Successful compilation → LLVM → run
    - Name/type resolution errors
    - Control-flow errors
(W) Unit tests:
    - NameResolver / TypeResolver
    - AST → IR lowering
    - IR → LLVM IR generation
(X) End-to-end tests: source → executable → expected output