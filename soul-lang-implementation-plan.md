# Implementation Plan: Making the Compiler Validate `soul-lang.md`

This document is the working plan for bringing the Soul compiler **frontend (tokenizer →
parser)** into conformance with the design in [`soul-lang.md`](./soul-lang.md). The
conformance corpus is [`testCompiler.soul`](./soul_tester/testCompiler.soul), which exercises
every section (§1–§20) of the spec.

> **Scope (agreed):** parser/tokenizer conformance *first*. Deep semantic checks
> (borrow checking, `AutoCopy`/move, `Send`/`Sync`, full name resolution for the new
> constructs) that the spec itself still marks **Open** are deliberately deferred. There is no
> codegen/backend yet, so this is frontend-only.

---

## 1. Current State

The compiler frontend already covers a substantial part of the spec:

- Import trees (`import Std.{Fmt, Display}`, `import ( ... )`)
- Variables & destructuring (`:=`, `:`, `mut`, tuple/named-tuple/constructor patterns)
- Assignments & compound assignment
- Structs, enums (incl. tuple & named union *variants*), typedef / `distinct`
- `use` blocks, `impl` blocks, inline single-method `impl`, fused `use X impl Y`
- Generics `<T>`, inline bounds `<T: Bound>`
- Arrays: `[N]T`, `[]T`, `[&]T`, `[&mut]T`, `[_]T`; literals, `[for ...]` fillers, `new[...]`
- Constructors `Type.(args)`, array constructors `Type.[...]`
- References `&x` / `&mut x`, pointers `*x`, deref
- `match`, match-method chain `.Variant{}` / `.else{}` / `.null{}`, `if`/`else if`/`else`
- `for` (infinite / while / foreach), `break`/`continue`/`return`
- Lambdas `(a, b) => expr`
- `f"..."` / `fstr"..."` string interpolation, `c"..."`
- `.copy` / `.pass` / `.sizeof` / `typeof` / `if type`-style match patterns
- `Res<T>`, `?T`, `RawPtr<T>`, `Error`, pointers, named variants `Foo.Bar`

All existing Rust tests pass (`cargo test` in `soul_ast/ast_parser` → 143 passing).

### What is NOT yet implemented (the gap)

This plan is organized as a set of **language-design differences** — each block below states
the spec (what `testCompiler.soul` expects), the current compiler behavior, and the change
needed.

---

## 2. Differences in Language Design (Spec vs. Current Compiler)

### 2.1 Keywords the spec needs but the tokenizer lacks

| Keyword(s) | Spec ref | Current behavior |
|---|---|---|
| `union` | §10 `union Literal { ... }` | No such term; unions only exist as enum *variants* |
| `async` | §15 `async fetchUser(...)`, `async main()` | Unknown token → parse error |
| `task` | §15 `task { ... }` | Unknown token → parse error |
| `spawn` | §15 `spawn { ... }` | Unknown token → parse error |
| `limit` | §12 `for ... limit 4`, §20 `type ... limit RANGE` | Unknown token → parse error |
| `intrinsic` | §16/§19.3 `intrinsic.array.toRaw<T>(...)` | Parsed as a plain ident; dotted chain not semantically modeled |
| `where` | §3.4 `where T: Display` | Token exists (`GenericWhere`) but errors at statement level; no where-clause parsing |
| `impl` | §3.4 `impl Display` in type position | Only used for `impl Trait { ... }` block; not valid as a type |
| `.await` | §15 `handleA.await` | Postfix access not modeled (like `.copy`/`.pass`) |

**Change:** add keyword tokens `Union`, `Async`, `Task`, `Spawn`, `Limit`, `Intrinsic`; keep
`Where`/`Impl`. Model `.await` as a postfix field-like access (matching the existing `.copy`
/ `.pass` / `.sizeof` handling), not a bare keyword.

### 2.2 Operators the spec uses that don't exist in the lexer

| Operator | Spec ref | Example | Current behavior |
|---|---|---|---|
| `**` (exponent) | §8, §11 | `this ** 2` | Not a token → error |
| `->` (map-chain) | §10.2 | `res->Ok{str.(it)}` | No token. **Conflict:** `>` already used (`RightArray`) to close generics `<...>` |
| `..` (range) | §12/§20 | `1..u8.MAX`, `0..3` | `DoubleDot` exists as `Range` operator, but range value syntax undefined |

**Change:**
- Add `**` as an operator with precedence above `*`/`/`.
- Add a distinct `->` (`RightArrow`) symbol for the map-chain. Distinguish `<`/`>` (generics)
  from `->` by lookahead: generics close with `>` and are followed by `(`/`{`; a map-chain is
  `->` immediately followed by `Ident{`. Confirm no ambiguity with the existing
  `LAMBDA_ARROW` (`=>`).
- `..` already tokenizes; formal `Range<T>` semantics are deferred (**Open** in spec).

### 2.3 Statements/declarations

#### 2.3.1 `union Name { ... }` — standalone union declaration (§10)
Spec:
```soul
union Literal {
    None,                              // unit
    Int(int),                          // tuple style
    Str{tag: str, value: str},          // struct style
}
```
Current: only `enum Name as T { Variant = expr, ... }` and tuple/named union *variants*
inside an enum.

Change: add a top-level `union` statement. Reuse the existing `EnumVariant::Union` machinery
(unit/tuple/named variants already parse). This is the same data model as an enum with union
variants but without a backing-type `as T`.

#### 2.3.2 Attributes `#[...]`, `#[!...]` (§9, §17)
Spec: `#[test]` on functions; `#[pass]` on a union variant; `#[!Copy]` / `#[!Eq]` / `#[!Ord]`
as culture-out markers.

Current: `ItemMetaData.attributes` always empty; `#` not part of the grammar.

Change: parse `#[ ident]` / `#[ ! ident]` prefixes before items (functions, fields, variants),
store the attribute name in `ItemMetaData`, and let the item continue parsing normally.
Minimal goal: `#[test]`-annotated functions parse without error.

#### 2.3.3 `async` prefix modifier on functions (§15)
Spec:
```soul
async fetchUser(id: int): str { ... }
```
Current: `const` / `pub` / `mut` are prefix modifiers; `async` is not valid.

Change: add `async` to the function-modifier path and record a flag on `FunctionSignature`.
`async main()` at the crate root runs the built-in runtime (no executor setup) — runtime is
out of scope; only parse it.

#### 2.3.4 `where` clause (§3.4)
Spec:
```soul
function<T>(value: T) where T: Display
```
Current: no where-clause parsing; `where` errors at statement level.

Change: after the parameter list (or after the return-type for the `=>` form), parse
`where Name: Bound, ...` and store the extra bounds on `FunctionSignature`. Also support the
inline bound inside `<...>` (already partially present) and the `impl Trait` anonymous form
(2.3.5).

#### 2.3.5 `impl Trait` anonymous generics in type position (§3.4)
Spec:
```soul
describeImpl(value: impl Display): str => ...
makeDefault(): impl Display => 0
```
Current: `impl` is invalid in type position.

Change: extend `try_parse_type` to accept `impl <TraitName>` (and dotted trait paths) as a
fresh anonymous generic bound. Works in parameter and return positions.

#### 2.3.6 Associated constants `::` inside bodies (§6, §8, §9)
Spec:
```soul
struct List<T> { LIST_GROW :: f32.(2) }
use Literal { VALUE :: 1 }
enum KeyWords as &str { ... }   // + FOR_STR :: "for" top-level
```
Current: `::` (`DoubleColon`) not handled as an associated-constant declaration inside struct/
use bodies.

Change: recognize `Ident :: <expr>` inside struct/use/enum bodies and record it as an
associated constant (a dedicated AST entry or reuse the variable mechanism).

#### 2.3.7 `type Name := T limit RANGE` (§20)
Spec:
```soul
type NonNullU8 := u8 limit 1..u8.MAX
```
Current: `type Name := T` and `type Name := distinct T` only.

Change: extend `parse_typedef` to accept an optional `limit <range>` clause after the RHS and
record it on `TypeDef`. `Limit<T, RANGE>`’s const-generic semantics are deferred (**Open**).

### 2.4 Expressions

#### 2.4.1 Structured concurrency `task {}` / `spawn {}` / `.await` (§15)
Spec:
```soul
task {
    handleA := spawn { fetchUser(1).await }
    handleB := spawn { fetchUser(2).await }
    userA := handleA.await
}
```
Change: add a `task { }` block statement and a `spawn { }` expression; model `.await` as
postfix access. Parse only — the runtime, `Send`/`Sync` checking, and "un-awaited handle is a
compile error" rule are out of scope.

#### 2.4.2 `limit N` on `for` (§12)
Spec:
```soul
for counter <= 0 limit 4 { counter -= 1 }
  .Err{panic("handle limit error")}
```
Change: add an optional `limit <expr>` to the `For` node; the loop expression can then be the
scrutinee of a follow-on match-chain (`.Err{...}`). The exact `Res` shape is deferred
(**Open**).

#### 2.4.3 Map-chain `->Variant{}` (§10.2)
Spec:
```soul
newRes := res->Ok{str.(it)}     // like Rust's res.map(...)
```
Change: add a new expression form distinct from `.Variant{}`. `->Variant{ transform }`
transforms only that one variant, passing others through; chaining is sequential. Parse as its
own node; semantic composition is name-resolution territory (deferred).

#### 2.4.4 `if type Pattern := expr` (if-let / type-match) (§10.2, §19.2)
Spec:
```soul
if type Err(message) := maybeErr { assertEq(message, "nope") }
if type v: int := value { println(f"int: {v}") }
```
Change: extend `if` parsing to accept a `type <Pattern> := <expr>` conditional that binds the
pattern inside the block. The match-pattern machinery already recognized `Type.Variant(...)`;
add the `if type` wrapper.

#### 2.4.5 `intrinsic.module.fn(...)` (§16, §19.3)
Spec:
```soul
ptr := intrinsic.array.toRaw<int>(buffer)
info := intrinsic.typeinfo(value.typeof)
```
`intrinsic` functions are not all unsafe — only raw-pointer ones need `unsafe { }`.

Change: model `intrinsic` as a namespaced call target (`intrinsic.` → submodule → `.fn<T>()`).
Because `.copy`/`.pass`/method-call resolution already handles dotted chains, this should parse
once `intrinsic` is a recognized name; confirm the dotted call chain with generics resolves.

#### 2.4.6 Bare `Type.` constructor-as-value (§3.5, §10, §12)
Spec:
```soul
[ str., str., str. ]               // array of "empty" str constructors
this.Int{f64.}                      // f64. applied to it inside a match arm
```
`Type.` (bare, no call) is a one-argument lambda equivalent to `el => Type.(el)`; inside a
match arm it implicitly applies itself to `it`.

Change: recognize `Ident.` (immediately before `,`, `]`, `}`, or `{`) as a constructor-as-value
expression rather than a field-access/parse error.

#### 2.4.7 Union / `Res` construction via call (§13)
Spec:
```soul
Ok(1), Err("bad"), Literal.Int(1)
```
Change: confirm `Type.Variant(...)` / `Ok(...)` / `Err(...)` parse as constructor calls, both
written paren-style and dot-style. Mark the exact interplay with the map-chain in 2.4.3.

#### 2.4.8 `for v in &mut values` element mutability (§12)
Spec:
```soul
for v in &mut values { v *= 2 }
```
Change: confirm the `&` / `&mut` collection forms set the element-binding modifier so an
element is mutable (`v *= 2` parses). A bare `for v in values` yields owned `int`; `&values`
yields `&int`.

---

## 3. Implementation Steps (execution order)

### Phase 1 — Tokenizer/lexer extensions
1. Add keywords: `Union`, `Async`, `Task`, `Spawn`, `Limit`, `Intrinsic`.
2. Add `**` operator (precedence above `Mul`) and `->` (`RightArrow`) symbol; resolve
   `->` vs `>` generics-close ambiguity by lookahead.
3. (Optional) tag `///` as a doc comment; not a parse blocker since `//` already skips the line.

### Phase 2 — Parser: statements/declarations
1. `union Name { ... }` (reuse union-variant machinery).
2. Attribute parsing `#[...]` / `#[!...]` → `ItemMetaData.attributes`.
3. `async` function modifier → `FunctionSignature`.
4. `where` clause → `FunctionSignature` bounds.
5. `impl Trait` in type position (parameter + return).
6. Associated constants `Ident :: expr` inside struct/use/enum bodies.
7. `type Name := T limit RANGE` on `TypeDef`.

### Phase 3 — Parser: expressions
1. `task { }` / `spawn { }` / `.await`.
2. `for ... limit N`.
3. Map-chain `->Variant{}`.
4. `if type Pattern := expr`.
5. `intrinsic.module.fn(...)`.
6. Bare `Type.` constructor-as-value.
7. `Ok(...)` / `Err(...)` / `Type.Variant(...)` construction calls.
8. `for v in &mut values` element mutability verification.

### Phase 4 — Conformance harness + tests
1. Point `soul_tester/config.json` `sourcePath` at `testCompiler.soul` (or `import` it from
   `main.soul`) so the tester compiles the whole smoke test; iterate to zero parse faults.
2. Add Rust regression tests in `soul_ast/ast_parser/src/tests/` for each new construct
   (`union.rs`, `async.rs`, `limit.rs`, `attributes.rs`, `map_chain.rs`, `where_clause.rs`,
   `assoc_const.rs`, `type_limit.rs`), asserting zero-severity parse, mirroring the existing
   `tests/mod.rs` style.
3. Keep the existing 143 parser tests green; run `cargo test` across all workspaces.

---

## 4. Explicitly Out of Scope (deferred / Open in spec)

- **Borrow checking semantics** — `AutoCopy`/`Copy`/`.copy` vs. move, use-after-move (the
  `// OPEN:` comments in `testCompiler.soul` call these out).
- **Name resolution** for the new keywords beyond parse level — `task`/`spawn` await rules,
  `Send`/`Sync` auto-derivation, `intrinsic` typing.
- **Goul** (§18) — "not designed yet".
- **Codegen / backend** — does not exist yet.
- **`Range<T>` formal type**, **const generics** for `Limit<T, RANGE>` (both **Open**).
- **Full operator precedence/associativity table** for `**` beyond the immediate need.

---

## 5. Acceptance Criteria

1. `testCompiler.soul` lexes and parses with **zero** error-severity faults through
   `soul_tester`.
2. Every spec example in `soul-lang.md` code blocks that touches the frontend parses.
3. New Rust unit tests cover each newly-supported construct.
4. All pre-existing 143 parser tests still pass.
