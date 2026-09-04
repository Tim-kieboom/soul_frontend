# The Soul Programming Language

Soul is a systems language with a Rust-style borrow checker and a terser, more expression-oriented
surface syntax. Its planned companion, **Goul**, is a transpile target that wraps heap values in
reference-counted boxes (ARC, à la Swift) for a runtime memory model instead of static borrow
checking — same language, two backends. Goul is not designed yet; see §18.

This document is both a reference and an introduction: it explains *why* a feature works the way
it does, not just its syntax, so it can be read cover-to-cover to learn the language or used as a
lookup reference. Anything not yet decided is marked **Open:** inline, and the full list is
collected in the Appendix.

---

## 1. Lexical Basics

- Files use the `.soul` extension.
- **Whitespace is not significant.** Soul is free-form like Rust or C, not newline-sensitive like
  Python. Writing one statement per line is just house style.
- **`;` has exactly one job: discard a value.** Since newlines don't separate statements, `;` is
  needed to stack multiple statements on one line (`a := 1; b := 2`), and it's also the one way
  to suppress a block's final expression from being that block's value (see §4).
- Three comment forms: `//` single-line, `///` doc comment, `/* ... */` multi-line.
- Punctuation used for declarations:
  - `::` — associated constant (`LIST_GROW :: f32.(2)`)
  - `:=` — local variable with inferred type (`x := 5`)
  - `:` — type annotation (`mut len: uint = 0`)
- **String interpolation**, two forms:
  - `f"{expr}"` — eagerly builds a `str`; any expression is allowed inside `{}`, not just bare
    names (`f"{el * 2}"`).
  - `fstr"{expr}"` — the lazy underlying form, mirroring Rust's `format_args!`: produces a
    "format arguments" value a function like `println` can consume directly, without forcing an
    allocation first. `f"..."` is sugar for building an `fstr"..."` and immediately materializing
    it into a `str`.

---

## 2. Modules & Visibility

Soul has one `import` keyword doing the job Rust splits across `use` and `mod` — there's no
separate module-declaration keyword.

```soul
import (
    Std.{Fmt, Display},
    Core.{
        Mem.{malloc, free},
        Array
    }
)
```

- Import trees nest with `.` as the path separator (Rust's `use a::{b, c::{d, e}}`, but with `.`).
- **Imports are scoped to where they're written**, following ordinary block-scoping: a top-level
  `import` is visible through the whole file; one written inside a function body is only visible
  in that function.
- **A module's visibility is its file name's capitalization.** Since there's no `mod`/`pub mod`
  distinction, the file itself carries that information: `Math.soul` (uppercase-leading) is a
  public module, `math.soul` (lowercase-leading) is private. This is file/module granularity
  only.
- **Item-level visibility** inside a file uses `pub`, `pub(crate)`, `pub(super)` — unaffected by
  the file-capitalization rule above. Private-by-default at the item level.

**Multi-file projects use a `Soul.toml` manifest.** A relative import (`import crate.sub`) finds
`sub.soul` next to the importing file within the same crate; a named import (`import Math`)
resolves against a dependency declared in `Soul.toml`:

```toml
name = "SoulTest"

[dependencies]
Std = {path = "lib/Std", linkage = "static"}
Core = {path = "lib/Core", linkage = "static"}
```

See `docs/crate-system-plan.md` for the compiler-side `CrateForest`/`.soulo` design this maps onto.

---

## 3. Types

### 3.1 Primitives
`int`, `uint`, `i64`, `u64`, `u8`, `f32`, `f64`, `bool`, `char`, `str`, `&str`, `none`

- `int`/`uint` are machine-width; explicit-width types (`i64`, `u8`, ...) are also available.
- `str` is an owned string; `&str` is a borrowed view (Rust's `String`/`&str` split).
- `none` is the unit/empty type. Since it has exactly one possible value, **any parameter of
  type `none` can be omitted from a call entirely** — a general calling-convention rule, not
  specific to any one construct.

### 3.2 Optionals
`?T` is an optional type (`[]?T` is an array of optionals). `.null{}` is a match-chain (§10)
specialized to the `null` case: the argument is the fallback for when the value is null, and the
value itself when it isn't.

```soul
option: ?int = null

// if null then 1 is returned
number: int = option.null{1}

// if null then panic is thrown
panicUnwrap: int = option.null{panic("panic message")}

// ignores null
//!!NEVER DO THIS UNLESS YOU REALLY KNOW IT CAN NEVER BE NULL!!
unsafe { unwrap: int = option.null{undefined} }
```

### 3.3 Arrays, Slices, and Array Literals

Array **types** — the size/borrow marker comes *before* the element type:
- `[N]T` — a fixed-size stack array (size is part of the type: `[4]int`).
- `[]T` — a heap array (dynamically sized).
- `[&]T` — a slice: a borrowed view over a run of `T`.
- `[&mut]T` — a mutable slice.

Array **literals** compose independently of stack-vs-heap:
```soul
a: [4]int := [1, 2, 3, 4]        // plain literal
a: [3]i64 := [i64: 1, 2, 3]      // explicit element-type prefix, for when inference needs help
a: [4]int := [for 4 => 0]        // fill/comprehension → [0, 0, 0, 0]
a: [3]int := [for i in 3 => i]   // named loop variable → [0, 1, 2]
a: []int  := new[1, 2, 3]        // new[...] allocates any of the above forms on the heap
```

### 3.4 Generics
`List<T>` — angle-bracket generics, with default type parameters supported:
`Res<O = none, E = str>`.

Bounds use Rust's syntax:
```soul
function<T: Display>(value: T)          // inline bound
function<T>(value: T) where T: Display  // where clause, for more complex bounds
function(value: impl Trait)             // anonymous generic parameter, no <...> needed
function(): impl Trait                  // works in return position too
```

### 3.5 Type-Call Construction Shorthand

A type name followed by `.(args)` or `.()` constructs a value — see §6 for the full story. Two
related shorthands:
- `Type.` (bare, no call) denotes **the constructor as a value** — a one-argument lambda
  equivalent to `el => Type.(el)`. Used directly inside a match-chain arm where a value (not a
  function) is expected, a bare `Type.` implicitly applies itself to `it`
  (`this.Int{f64.}` ≡ `this.Int{f64.(it)}`).
- `Type.EMPTY`/similar named constants (e.g. `str.EMPTY`) are ordinary associated constants,
  unrelated to the `Type.` shorthand above — they just happen to coincide for `str`.

---

## 4. Expressions, Blocks, and Statements

**Soul follows Rust's block-expression model: everything is an expression, and a block's last
expression is its value unless suppressed by `;`.** This applies uniformly — `if`, `match`,
function bodies, and `for` loops with `limit` (§12) all follow the same rule, not just function
bodies:

```soul
x := if cond { 1 } else { 2 }         // if/else as an expression — both arms' types must agree
y := match value { Ok(v) => v, Err(_) => 0 }

doSomething(): int => 1

returnsNone() {
    println("hello")
    doSomething();   // trailing `;` discards the value — block now evaluates to `none`
}

returnsInt(): int {
    println("hello")
    doSomething()    // no trailing `;` — block evaluates to int
}
```

**Items can be nested inside other items.** Functions can be declared inside functions, and
presumably structs/enums/unions/traits can be declared locally too. A **named** function or
method, wherever it's declared, **never captures its enclosing scope** — exactly like Rust's
`fn`. Only an **anonymous closure literal** (`(params) => expr`) captures:

```soul
name() { ... }            // a normal, non-capturing function — even if nested inside another
() => 1 + 1                // an anonymous closure literal — CAN capture the enclosing scope
```

Closures follow Rust's model: capture mode (borrow / mutable borrow / move) is inferred from
usage, with an explicit `move` to force capture-by-move. Closure types are presumably a
`Fn`/`FnMut`/`FnOnce`-equivalent family, usable via `impl Trait` (§3.4).

---

## 5. Functions

**Soul has no `fn` keyword.** A function is just its name and parameter list:

```soul
add(a: int, b: int): int => a + b
```

`const`, `pub`, and `async` (§15) are prefix modifiers on that same bare form — there's no `fn`
keyword for them to modify:

```soul
const comptimeAdd(a: int, b: int): int => a + b     // comptime-evaluable
pub validate(input: str): bool => input.len() > 0   // => sugar for a single-expression body
async fetchUser(id: int): str { ... }               // §15
```

**No ad-hoc overloading, anywhere — with one deliberate exception.** A given function/method
name resolves to exactly one signature; Soul doesn't dispatch on argument types or count the way
C++/Swift do. **The one exception is the `This.(name: T)` constructor form (§6):** a struct can
declare several of these, each distinguished by its single parameter's type, and the compiler
dispatches on the argument's type at the call site. This is a narrow, special-cased carve-out
for construction specifically — not a general overloading mechanism — and it's the same
type-dispatch principle behind the array-literal constructor `This.[T](param)` (§6) accepting
multiple element types too.

---

## 6. Structs

```soul
pub struct List<T> {
    LIST_GROW :: f32.(2)

    mut len: uint = 0
    mut buffer: []?T = []

    pub This.() => This{..}
    pub This.[int](array) => This{ buffer: array, len: array.len() }

    pub len(&this): uint => this.len
}
```

- `pub` on the struct and per-member controls visibility (§2).
- Fields default to **immutable**; `mut` opts a field into mutability.
- Field default values (`= 0`, `= []`) fill in whatever a constructor doesn't set.

**Construction has three independent mechanisms**, chosen by shape:

1. **`This.() => This{..}` — the zero-argument/default constructor.** Its own dedicated syntax.
   `This{..}` is the struct-literal spread, filling every field from its default value.
2. **`This.(name: T) => This{..}` — construct from a value of type `T`.** A struct can declare
   several of these side by side, each with a differently-typed single parameter; the compiler
   dispatches on the argument's type at the call site (the one overloading exception, §5):
   ```soul
   pub struct Number {
       mut inner: f64
       This.(float: f64) => This{inner: float}
       This.(interger: int) => This{inner: f64.(interger)}
   }

   mut number := Number.(int.(1))     // one-arg call → the `int` constructor
   number = Number.(f64.(2))          // one-arg call → the `f64` constructor
   assertEq(number.typeof, Number)
   ```
   `Type.(arg)` dispatches on `arg`'s type to the matching `This.(name: T)` definition. No-args
   always means `This.()`; one-arg always means this type-dispatched form — the two are cleanly
   split by arity. This is also the real mechanism behind primitive conversions like `f32.(2)`,
   `u8.(1)`: built-in constructors the language ships for its own primitive types.
3. **`This.[T](param)` — the array-literal constructor.** Its own mechanism, since it routes a
   *literal syntax form* rather than converting a single value. **A struct can declare more than
   one, for different element types**, following the same type-dispatch principle as (2):
   ```soul
   struct IntArray {
       array: []int
       len: uint

       This.[int](array) => This{
           len: array.len(),
           array,
       }

       This.[u8](array) => This{
           len: array.len(),
           array: array.intoIter().map(int.).toArray(),
       }
   }

   mut array := IntArray.[1, 2, 3, 4]   // routes to This.[int](array)
   array = IntArray.[1_u8, 2, 3, 4]     // routes to This.[u8](array)
   assertEq(array.typeof, IntArray)
   ```

**Fallibility is signaled by the return type, with no separate keyword.** Every constructor
above implicitly returns `This` — none of them declare a return type at all. A constructor that
*can* fail (like `Limit<T, RANGE>`'s, §20) must say so by explicitly declaring
`: Res<This>` (or `: ?This`):
```soul
This.() => This{..}                          // infallible — no return type, always succeeds
This.(value: T): Res<This> => ...             // fallible — explicit Res<This> return type
```
This is the general rule for constructors, not something specific to `Limit`: an **omitted
return type means infallible**, and the only way to be fallible is to opt in by writing the
`Res<This>`/`?This` annotation out — the same way any other function already signals whether it
can fail via its declared return type (§13), just applied consistently to constructors too.

Regular methods use `&this`/`&mut this` receivers (§11), `=>` for single-expression bodies, `{ }`
for multi-statement ones.

---

## 7. Traits and `impl`

Soul supports **two `impl` spellings**, chosen by how many methods a trait needs:

```soul
use Type {
    // single-method inline form — trait named up front, no nested block
    impl Index<Out = T> index(&this, index: uint): &This.Out {
        value := &this.buffer[index]     // bounds-checked automatically
        unsafe { value.null{undefined} }
    }

    // block form — for multi-method traits
    impl MyTrait {
        method1() {}
        method2() {}
    }
}
```

Both forms work either inside a type's own body, or inside a `use` block extending a type from
outside its definition (§8). `use` and `impl` can also **fuse into one header** when the
extension's only content is that one conformance:

```soul
use Literal impl Display {
    fmt(&this, &mut f: &mut Formatter) { ... }
}
```

**A struct can implement the same generic trait multiple times if the generic parameters
differ** — including when the parameter is in output position (`Index<Out=T>`), not just input
position. Disambiguation for output-position cases comes from context/return-type inference at
the call site. What's disallowed is implementing a trait with *identical* generic parameters
twice.

---

## 8. `use` Blocks and Extension

```soul
use Literal {
    VALUE :: 1

    tryIntoFloat(this): ?f64 {
        this.Int{f64.}.Uint{f64.}.Float{it}.else{null}
    }
}
```
`use Type { ... }` adds constants/methods to a type from outside its definition (Swift's
`extension`). Free-standing `Type.method(...)` (no `use` block) extends *any* type, including
primitives, without ceremony:
```soul
int.square(this): int => this ** 2
```
The first parameter doesn't have to be `this` — a name other than `this` makes it a namespaced
function under `Type` rather than an instance method, still called through `Type.name(...)`:
```soul
int.parse(value: &str): Res<int> { ... }   // called as int.parse("42"), not "42".parse()
```

---

## 9. Enums

`enum Name as BackingType { Variant = const expression, ... }`. `.value` gives the backing value
from the compile-time expression; `.tag` gives the ordinal index. The compiler auto-generates
`fromValue`/`fromTag` reverse lookups, both returning an optional (`null` if nothing matches).

```soul
FOR_STR :: "for"
FIRST_TAG :: 0_u32

enum KeyWords as &str {
    ForLoop = FOR_STR,
    InForLoop = "in",
    TypeMatching = "match",
}

#[test]
test_basic_enum_features() {
    variant := KeyWords.ForLoop
    assertEq(variant.typeof, KeyWords)
    assertEq(variant, KeyWords.ForLoop)

    string := variant.value
    tag := variant.tag
    assertEq(string, FOR_STR)
    assertEq(tag, FIRST_TAG)

    fromValue := KeyWords.fromValue(FOR_STR)
        .null{panic(f"KeyWords.fromValue({FOR_STR}) should not be null")}

    fromTag := KeyWords.fromTag(FIRST_TAG)
        .null{panic(f"KeyWords.fromTag({FIRST_TAG}) should not be null")}

    assertEq(variant, fromTag)
    assertEq(variant, fromValue)
}
```

---

## 10. Unions and Pattern Matching

### 10.1 Declaring a union

```soul
union Literal {
    None,
    Int(int),
    Str{tag: str, value: str},
}

parse_int(value: &str): int {
    int.parse(value).Err{panic("parse failed")}
}

#[test]
test_basic_union_features() {
    mut variant := Literal.None
    variant = Literal.Str{tag: "f".copy, value: "hello".copy}
    variant = Literal.Int(1)
    assertEq(variant.typeof, Literal)
    assertEq(variant, Literal.Int(1))

    number := variant
        .None{0}
        .Int{it}
        .Str{parse_int(it.value)}
}
```
Variants can be unit (`None`), tuple-style (`Int(int)`), or struct-style with named fields
(`Str{tag: str, value: str}`) — all three can coexist in one union.

### 10.2 Three ways to inspect a union

**The match-chain (`.Variant{}`)** — a `match` expression spelled as a chain. Each
`.Variant{ body }` is one arm; the whole chain *is* the match, not a sequence of steps:

```soul
Literal.tag(&this): &str {
    this
        .None{"none"}
        .Int{"int"}
        .Str{"str"}
}
```
This desugars to `match this { None => "none", Int(x) => "int", Str(s) => "str" }`.
- The payload binds to `it` by default, or an explicit name: `this.Int{x => x + 1}`.
- Chains are **never required to be exhaustive.** Any variant left out passes through
  implicitly — `.else{}` overrides that default rather than satisfying a requirement, and it
  still binds the passthrough payload as `it`, so it can transform rather than just replace it:
  ```soul
  value: Res<int> = Ok(1)

  number := value.Err{1}.Ok{it + 1}   // ≡ match value { Err(_) => 1, Ok(val) => val + 1 }
  number := value.Err{1}              // ≡ match value { Err(_) => 1, other => other }
  number := value.Err{1}.else{it + 1} // ≡ match value { Err(_) => 1, other => other + 1 }
  ```
- Chain arms are **read-only, by-value** — no `&mut` access to a payload through a chain.

**The map-chain (`->Variant{}`)** — a completely different thing: single-variant `map`/`map_err`,
Rust-style, applied one at a time rather than as a combined match:
```soul
res: Res<int> = Ok(1)
newRes := res->Ok{str.(it)}   // like Rust's res.map(|el| el.to_string())
assertEq(newRes.typeof, Res<str>)
```
Each `->Variant{}` only transforms its one variant, passing every other variant through
unchanged; chaining several is sequential, not a single expression like `.` chains are.

**The traditional block `match`** — Rust's `match`, used directly, for nested/multi-value
patterns the chain forms can't express:
```soul
match (resultA, resultB) {
    (Ok(a), Ok(b)) => a + b,
    (Err(e), _) => panic(e),
}
```
This one *is* exhaustive, with Rust's full pattern grammar (tuples, guards, bindings).

**If-let**: `if type Err(err) := newRes { ... }`, binding `err` only inside the block.

**`typeof`**: `newRes.typeof` returns a first-class, comparable type value.
**Open:** compared at compile time, runtime, or both (this affects monomorphization)?

---

## 11. Ownership & Borrowing

- **Move-by-default.** Assigning or passing a value by value moves it; the old binding becomes
  invalid, exactly like Rust.
- **Aliasing: strict, static, Rust-style.** At any point a value is borrowed by exactly one
  `&mut`, or by any number of `&`, never both. Violations are compile errors.
- **Lifetimes: elided by default; Rust's `'a` syntax directly for the rare explicit case**
  (struct fields holding borrows, some higher-order signatures). No new sigil invented.
- **`&this`/`&mut this`** are borrowed/mutably-borrowed method receivers; free-standing
  `&x`/`&mut x` generalize the same rule to any binding. `[&]T` (a slice) is itself a borrow,
  subject to the same aliasing rules.

**Two duplication mechanisms, mirroring Rust's `Clone`/`Copy` split:**
- **`.copy` — a keyword, not a method call, matching `.await` (§15).** Written without
  parentheses: `b := a.copy`, not `a.copy()`. It performs a compiler-provided, independent
  duplicate of any type, on by default. Opt out with `#[!Copy]` for types where duplication
  shouldn't be allowed at all (a file handle, a mutex guard, ...).
- **`AutoCopy` (≈ Rust's `Copy`) — a real trait, *not* implemented by default; opt-in only.** A
  type implementing `AutoCopy` gets *implicit* copy-on-assignment (`a := b` duplicates rather
  than moves). Opt in with `use Type impl AutoCopy {}`. Primitive scalars are the built-in
  `AutoCopy` implementers.

  This establishes the language's **general derivation pattern**: `#[!Trait]` suppresses a
  default-on capability; `use Type impl Trait {}` opts into one that isn't on by default.

**`Drop`**, for custom cleanup, written with the single-method inline `impl` form:
```soul
impl Drop drop(&mut this) { free(this.buffer) }
```
Called automatically at scope-exit for a value's final (non-moved-from) owner.

**Open:** what `mut` means for a variable after it's been moved from (is reassignment always
legal regardless, since the slot is empty)?

---

## 12. Control Flow

**`for` unifies Rust's `loop`/`while`/`for`** — one keyword, disambiguated by what follows it.

```soul
for { println("loop") }              // infinite loop — type is ! (never); no break-with-value

mut counter := 5
for counter <= 0 { counter -= 1 }     // while-loop: bare condition, no `in`

for el in &array { ... }             // el: &int      — like .iter()
for el in &mut array { ... }         // el: &mut int  — like .iter_mut()
for i, el in array { ... }           // el: int, indexed — like .into_iter().enumerate()
```

**`limit N` applies to every `for` form** — bare infinite, conditional, or for-each — capping
iterations. Exceeding the cap breaks out **with an error** rather than running forever, and the
whole construct becomes an **expression** evaluating to a `Res`, chainable directly:
```soul
for counter <= 0 limit 4 { counter -= 1 }
.Err{panic("handle limit error")}
```
This also gives an otherwise-infinite `for {}` a way to become finite and typed, without needing
`break value` (which doesn't exist in Soul — `break` never carries a value).

**Open:** exact `Res` shape from `limit`; whether `break`/`continue` exist as plain
(valueless) keywords and whether labeled loops exist; range syntax (`0..3` vs `0..=3`) is
inferred but not formally specified.

---

## 13. Error Handling

```soul
union Res<O = none, E = str> {
    Ok(O),
    #[pass]
    Err(E),
}
```
- **`.pass` is Soul's spelling of Rust's `?`.** It unwraps the non-`#[pass]` variant's payload,
  or early-returns the `#[pass]`-marked variant from the enclosing function. `#[pass]` marks
  which variant is the "propagate outward" case, generalizing beyond `Res` to any
  two-or-more-variant union with exactly one `#[pass]`-marked variant. Using `.pass` where the
  enclosing function's return type isn't compatible is a compile error, with no special-casing
  for `main`.
- **`assert` panics on failure.** `panic("message")` is directly callable too, independent of
  `assert` — a failed assert is sugar for "check the condition, panic if false." Panics unwind
  by default (running `Drop`s), with an abort mode available at build-configuration level, same
  as Rust.

So Soul has two tiers: `Res`/`Option` (`?T`) + `.pass` for recoverable, expected errors;
panics for programmer-error/unrecoverable conditions.

---

## 14. Operators and Auto-Derived Traits

- **`Eq`/`Ord` are auto-derived whenever every field/variant supports them** — structural
  equality/ordering, field-by-field or variant-by-variant, automatic rather than requiring a
  derive annotation. `#[!Eq]`/`#[!Ord]` presumably opt out.
- **Custom operator overloading is planned, not yet designed.** The intent is `+`/`-`/`==`/`<`
  etc. all dispatching through traits the same way `Index` already does (`Add`, `Sub`, ...),
  likely following the same "differ by generic parameter" rule already established for traits.

**Open:** the full operator list and precedence/associativity table don't exist yet; `Ord`
tie-breaking on multi-variant unions is unspecified.

---

## 15. Concurrency: `async`/`await`

Soul deliberately combines pieces of Rust and Kotlin rather than copying either wholesale:

- **A built-in runtime, zero setup** — no executor crate to choose or configure; `async main()`
  just runs.
- **`async` stays an explicit keyword — function coloring included, deliberately.** Async
  functions can't be casually called from sync code and vice versa, matching Rust. This is a
  conscious trade: the ergonomics goal is removing *setup* and *lifetime* pain, not the
  sync/async boundary itself.
- **Structured concurrency — a spawned task's lifetime is tied to its spawning scope**, closing
  Rust's biggest ergonomic gap around spawning: no `'static`/`Arc` needed just to borrow local
  data into a spawned task, because the scope structurally can't end before its children do.

```soul
task {
    handleA := spawn { fetchUser(1).await }   // handleA: Task<str>
    handleB := spawn { fetchUser(2).await }

    userA := handleA.await   // both fetches ran concurrently
    userB := handleB.await
    println(f"{userA}, {userB}")
}
```

- **`task { }`** opens a structured scope; it doesn't complete until every `spawn` inside it has.
- **`spawn { }`** launches concurrent work and returns a `Task<T>` handle.
  - The handle **must be `.await`-ed inside the enclosing `task {}` block** (not deferred until
    after it closes).
  - **An un-awaited handle is a compile error** — the same spirit as Rust's `#[must_use]`.
  - **A panicking spawn surfaces through `.await` as `Res<T, PanicError>`**, not by crashing
    — a genuine asymmetry with directly-awaited calls, which still propagate panics normally,
    since a spawn has no synchronous call stack to unwind through.
- **Ordinary borrow-checker rules are the data-race rules** — two `spawn`s each taking `&data` is
  fine; two wanting `&mut data` hits the same aliasing violation as single-threaded code, just
  checked across `spawn` boundaries too.
- **`Send`/`Sync`, same jobs as Rust's:** moving a value into a `spawn` needs `Send`; sharing one
  by reference across multiple `spawn`s needs `Sync`. Both auto-derived-when-possible, following
  the same `#[!Trait]`/`use ... impl Trait {}` pattern as `Copy`/`AutoCopy`.

**Open:** detached/fire-and-forget tasks (spawn that outlives its scope), cancellation, channels
and a `select`-equivalent, and whether an `actor`-style construct exists (it would collide with
the coloring decision — would calling an actor method need `.await`?).

---

## 16. `unsafe` and Intrinsics

```soul
unsafe {
    ptr := toRaw<T>(this.buffer)
    toSlice(ptr, this.len())
}
```
`intrinsic.{...}` is a pseudo-module for compiler-provided primitives — but **`intrinsic`
functions aren't all unsafe**. Only the ones that are actually dangerous (raw-pointer
manipulation like `toRaw`/`toSlice` above) need to be called inside an `unsafe { }` block;
others (e.g. `intrinsic.typeinfo`, §19) are perfectly safe and callable directly, with no
`unsafe` wrapper needed. `unsafe { }` gates specific dangerous operations, not the `intrinsic`
namespace as a whole.

---

## 17. Testing

No special `Test` trait. Testing is a file-naming + attribute convention:
- A source file named `test{Name}` (e.g. `testMath.soul`) is a test module.
- `#[test]` marks an individual test function:
  ```soul
  #[test]
  addsCorrectly() {
      assert(comptimeAdd(2, 3) == 5)
  }
  ```

**Open:** exact `soul test` CLI behavior; whether a failed `assert` is caught per-test rather
than aborting the whole run (presumably yes).

---

## 18. Goul (Planned)

Not designed yet. The intended relationship: same syntax as Soul, but heap values are
automatically wrapped in reference-counted boxes, and the borrow checker either turns off,
stays on for cross-thread safety only, or some hybrid — undecided. Interop between Soul and Goul
modules in one project is also open.

---

## 19. Reflection: `any` and `TypeInfo`

Odin-style reflection: a value can be erased to a type that still knows what it actually is at
runtime, and that runtime type can be inspected in detail. Soul already had half of this in
place before it was designed — `.typeof` (§10) returning a first-class, comparable type value is
exactly Odin's `typeid`, just under a different name.

### 19.1 `any` is unsized — like Rust's `str`, you can only ever hold `&any`

**`any` can never be owned directly, the same way Rust's `str` can only exist behind a pointer
(`&str`, never a bare `str` local).** There's no implicit borrowing — the `&` is always written
explicitly, both in the type and at the call site, consistent with every other borrow in the
language:

```soul
printAny(value: &any) {
    if type v: int := value {
        println(f"int: {v}")
    } else if type v: str := value {
        println(f"str: {v}")
    } else {
        println(f"something else: {value.typeof}")
    }
}

x := 42
printAny(&x)
```

Because `any` is inherently non-owning, forcing it to only ever appear as `&any` makes that fact
visible directly in the type instead of being a hidden property of an otherwise-ordinary-looking
`any` — the same reasoning Rust applies to `str`. This also means **no special lifetime syntax
is needed for `any`** — it's just an ordinary borrow, so the usual `&'a` form already covers the
case of storing one in a struct field, with no `any<'a>`-style special case required:

```soul
struct Container<'a> {
    value: &'a any
}
```

### 19.2 Downcasting reuses `if type`

No new operator — `if type Pattern := expr` (already used for union-variant matching, §10) is
extended to accept a plain `name: Type` binding, tried against an `&any`'s runtime type:

```soul
if type v: int := value {
    // v is in scope here, only if `value` currently holds an int
}
```

Since `any` never owned the data, the bound `v` is itself a borrow of the underlying value
(`v: &int` here) — consistent with §19.1. For a type that implements `AutoCopy` (every
primitive does, §11), that borrow behaves like an implicit copy wherever one is needed, the same
as any other `AutoCopy` value would. **Open:** whether downcasting to a non-`AutoCopy` type
(a struct, say) yields only `&T`, or whether an owned copy is obtainable via `.copy` (§11) on
the bound reference — `v.copy` — the same way you'd duplicate any other borrow.

### 19.3 Full reflection: `intrinsic.typeinfo` and `TypeInfo`

Beyond just checking "is this an int," the full structural breakdown of a type — struct fields,
union variants, array element types, and so on, mirroring Odin's `Type_Info` union — comes from
an explicit intrinsic call rather than a chained property:

```soul
info := intrinsic.typeinfo(value.typeof)
```

**No `unsafe` needed** — `intrinsic.typeinfo` is a safe intrinsic (§16); only the intrinsics
that actually touch raw memory require `unsafe { }`.

```soul
union TypeInfo {
    Primitive(PrimitiveKind),
    Struct{ fields: []FieldInfo },
    Union{ variants: []VariantInfo },
    Enum{ backing: TypeInfo, variants: []EnumVariantInfo },
    Array{ elem: TypeInfo, len: ?uint },   // len is none for a heap []T
    Borrow{ inner: TypeInfo, mutable: bool },
}

struct FieldInfo {
    name: str
    type: TypeInfo
    offset: uint
}
```

This two-tier split — a cheap, comparable `typeid`-like value from `.typeof` itself, versus the
full structural breakdown only computed when `intrinsic.typeinfo` is actually called — mirrors
Odin's `typeid`/`Type_Info` split and means you don't pay for reflecting a type's full shape
unless you ask for it:

```soul
printFields(value: &any) {
    match intrinsic.typeinfo(value.typeof) {
        Struct(s) => for field in s.fields {
            println(f"{field.name}: {field.type}")
        }
        Primitive(p) => println(f"primitive: {p}"),
        _ => println("not a struct"),
    }
}
```

**Open:** exact shape of `PrimitiveKind`/`VariantInfo`/`EnumVariantInfo`; whether `TypeInfo` is
itself usable as an `&any`'s target for `if type` (reflecting on reflection); whether field
*values* (not just names/types/offsets) are reachable generically for a struct behind an `&any`
— that's the piece that would make something like a generic serializer possible, and it's the
part most likely to run into the borrow-checker/ownership questions from §19.1 and §19.2 again,
just per-field instead of for the whole value.

---

## 20. Type Aliases: `type`, `distinct`, and `limit`

```soul
// typedef
type Byte := u8

// only allows numbers in range 1..u8.MAX
type NonNullU8 := u8 limit 1..u8.MAX

// distinct type
type Number := distinct f64

number := Number.(f64.(0))
double := f64.(number)
```

Three related but distinct forms:

**`type Name := T` — a transparent alias.** `Byte` and `u8` are fully interchangeable
everywhere — same representation, same type as far as the compiler's concerned. This is purely
for readability, like Rust's `type` or a C `typedef`.

**`type Name := distinct T` — a nominal, opaque wrapper.** `Number` and `f64` are *not*
interchangeable, even though `Number` has exactly `f64`'s representation underneath. Converting
between them requires going through construction, the same `Type.(arg)` dispatch mechanism
already used everywhere else (§6) — the compiler auto-generates the wrap (`Number.(value: f64)`)
and unwrap (`f64.(value: Number)`, an additional constructor overload on `f64` itself) the same
way it auto-generates `From`-style conversions for any other type-dispatched constructor. This
is Odin's `distinct` directly.

**`type Name := T limit RANGE` — sugar for a real generic wrapper type, `Limit<T, RANGE>`.**
`NonNullU8` is really `Limit<u8, 1..u8.MAX>` under the hood — not a separate mechanism from
`distinct`, but arriving at the same nominal-separation behavior *because* `Limit<T, RANGE>` is
a genuine wrapper struct, not a bare alias. Its constructor is declared fallible the same way any
other constructor would be (§6) — an explicit `Res<This>` return type, nothing `Limit`-specific:
```soul
struct Limit<T, RANGE> {
    value: T
    This.(value: T): Res<This> => ...   // explicit Res<This> — this constructor can fail
}
```
which is exactly why it can be called like this:
```soul
value: Res<NonNullU8> = NonNullU8.(0)     // Err — 0 is outside 1..u8.MAX
value: Res<NonNullU8> = NonNullU8.(5)     // Ok(NonNullU8(5))
```

**Construction returns a `Res` rather than panicking** — matching `for ... limit N`'s
Result-producing behavior (§12), so both uses of the `limit` keyword in the language share the
same "bounded, and the violation is recoverable" shape rather than one panicking and the other
not.

This introduces a mechanism the rest of the spec hasn't needed yet: **`RANGE` in
`Limit<T, RANGE>` is a compile-time *value*, not a type** — a const generic parameter. Up to
this point every generic parameter in the language (`List<T>`, `Result<O, E>`, ...) has been a
type. `Limit` needs its second parameter to be a range value instead, so const generics are now
a real prerequisite, not just a `Limit`-specific detail.

**Open:**
- Const generics themselves aren't designed — syntax for declaring one (`Limit<T, const R: Range<T>>`?
  something else?), and how broadly they're allowed beyond this one use case.
- `Range<T>` (or whatever type a `1..u8.MAX` expression actually has) isn't formally specified
  anywhere — this connects directly to §12's still-open range-syntax question.
- How to get the underlying `T` back out of a `Limit<T, RANGE>` — a `.value` field, an unwrap
  conversion mirroring `distinct`'s `T.(value: Limit<T, RANGE>)`, or something else?
- Whether a `Limit<T, RANGE>`'s value can be mutated in place after construction (risking
  re-violating the range) or whether it's immutable once built, requiring reconstruction
  (and a fresh `Res` check) to change.

---

## Appendix: Open Design Questions

Collected from inline **Open:** markers above, for at-a-glance status:

- **Pattern matching** — the exact type-inference rule for a partial `.` match-chain (handled-arm
  type vs. the implicit passthrough-arm type); whether `if type Pattern := &mut expr` is the
  answer for "match and mutate".
- **Control flow** — `Res` shape from a `limit`-capped loop; whether `break`/`continue` exist
  as plain keywords and whether labeled loops exist; formal range syntax (`0..3` vs `0..=3`).
- **Ownership** — what `mut` means for a variable after it's been moved from.
- **Traits** — output-position generic disambiguation when there's no type annotation to infer
  from (e.g. `Index<Out=T>` with no surrounding context).
- **Operators** — full list, precedence/associativity table, and the overloading design itself.
- **Concurrency** — detached tasks, cancellation, channels/`select`, and whether an `actor`
  construct exists alongside `async`/`await` coloring.
- **Misc** — whether a semicolon is ever *required* beyond the cases already settled in §1/§4;
  whether the implicit-apply-to-`it` sugar for bare `Type.` generalizes to any unapplied
  single-argument callable or stays specific to constructors.
- **Reflection** — whether downcasting an `any` to a non-`AutoCopy` type can yield an owned
  `.copy` or only a borrow; exact shape of `PrimitiveKind`/`VariantInfo`/`EnumVariantInfo`;
  whether generic per-field *value* access through an `any` is possible (needed for a generic
  serializer, and the hardest open piece of §19).
- **Goul** — essentially everything (§18).
- **Type aliases** — const generics aren't designed at all (needed for `Limit<T, RANGE>`);
  `Range<T>`'s formal type is unspecified (ties to §12's open range-syntax item); how to unwrap
  a `Limit<T, RANGE>` back to `T`; whether its value is mutable in place or reconstruction-only.
