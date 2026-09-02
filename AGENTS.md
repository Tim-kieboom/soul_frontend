# AGENTS.md

Guidance for AI coding agents working in this repository (a Rust project). This file
consolidates the project's coding and review guidelines so agents behave consistently
whether writing, editing, or reviewing code.

## Priorities

When writing Rust for this project, optimize for, in order:

1. Correctness
2. Readability
3. Maintainability
4. Performance (only after measuring)

## Before writing code

- Think about ownership.
- Think about API design.
- Think about visibility.
- Prefer simple solutions.
- When multiple valid implementations exist, choose the one that is easiest to understand.

## Coding rules

### Visibility

Prefer the narrowest visibility that works:

```
private → pub(super) → pub(crate) → pub
```

Never use `pub` unless required.

### Ownership

- Borrow instead of clone.
- Avoid unnecessary allocations.
- Reserve capacity when size is known.
- Prefer immutable references.
- Avoid `Rc`, `Arc`, and interior mutability unless required.
- Never clone just to satisfy the borrow checker.

### Functions

Functions should:

- have one responsibility
- use early returns
- avoid deep nesting
- avoid boolean parameters
- avoid excessive parameters (>6)

Prefer `let Some(value) = option else { return Err(...); };` over nested `if`.

### Types

Prefer:

- enums over booleans
- strong types over primitives
- structs with named fields
- composition over inheritance
- generics / `impl Trait` over `dyn Trait`

Avoid public mutable fields.

### Error handling

Use `Result`, `Option`, `?`, `let ... else`, `match`.

Avoid `unwrap()`, `expect()`, `panic!()` unless violating an internal invariant.

Use concrete error types. Only use `anyhow::Result` for application entry points or
orchestration.

### Performance

Never optimize without evidence. Prefer, before micro-optimizations:

- good algorithms
- fewer allocations
- fewer clones
- cache-friendly data
- stack allocation

### Unsafe

Unsafe code is only acceptable when:

- safe Rust cannot express it
- measurable benefit exists
- invariants are documented

Every `unsafe` block requires a `SAFETY` comment.

### Style

Prefer explicit code over clever code. Avoid complicated iterator chains when explicit
control flow is easier to understand. Readable code always wins.

### Standard APIs

Prefer Rust conventions: `new`, `Default`, `From`, `TryFrom`, `AsRef`, `Into`, `Iterator`,
`IntoIterator`. Do not invent custom APIs where a standard trait fits.

### Testing

- Write unit tests in separate test files, not inline `mod tests` blocks.
- Place each module's tests in a `{name}_tests.rs` file next to the source
  (`src/foo.rs` → `src/foo_tests.rs`) and declare it with
  `#[cfg(test)] #[path = "foo_tests.rs"] mod tests;` in the source file, so tests keep
  access to private items.
- Test public behavior and edge cases, not implementation details.
- Keep test helpers small and focused.
- Use temporary directories (e.g. `tempfile::tempdir()`) for filesystem tests instead of
  writing into the repo.
- `unwrap()` is acceptable in tests when the failure cannot happen without a test bug.
- Name tests after the behavior under test.

### Output expectations

Generate production-quality Rust. Do not explain obvious Rust concepts. If a requested
implementation violates these guidelines, explain why and produce a guideline-compliant
alternative instead.

## Code review checklist

When reviewing Rust code in this repo (e.g. via `/code-review`), check against these
guidelines and do not rewrite code unless requested. Review dimensions:

- **Correctness** — bugs, edge cases, invalid assumptions, ownership mistakes.
- **Readability** — unnecessary complexity, clever code, nested control flow, difficult
  iterator chains, poor naming.
- **API design** — excessive visibility, poor encapsulation, public implementation
  details, unnecessary generics, poor constructors.
- **Ownership** — unnecessary clones/allocations, ownership that could be borrowing,
  misuse of `Rc`/`Arc`, unnecessary interior mutability.
- **Types** — weak type design; prefer enums over booleans, strong types over
  primitives, composition, `impl Trait`/generics, named structs.
- **Error handling** — `unwrap()`/`expect()`/`panic!()`, hidden errors, poor `Result`
  usage, generic `String` errors; suggest concrete error types.
- **Performance** — unnecessary allocation, repeated cloning, missing `reserve()`,
  inefficient collections. Do not recommend micro-optimizations without measurable
  benefit.
- **Style** — early returns, explicit control flow, standard Rust naming, function
  size, parameter count, visibility.
- **Testing** — public behavior/edge case coverage, tests in `{name}_tests.rs` files
  (not inline), tests not brittle or coupled to implementation details, `unwrap()`
  avoided where an assertion is feasible, filesystem tests use temp dirs.
- **Unsafe** — every block justified, has a `SAFETY` comment, preserves invariants.

### Severity levels

- **Critical** — likely bug or memory issue.
- **Major** — violates project architecture or significantly hurts maintainability.
- **Minor** — readability, style, or small API issue.
- **Suggestion** — possible improvement with no correctness impact.

### Review output format

For every issue report:

- Severity
- Guideline violated
- Explanation
- Suggested improvement

If the code follows the guidelines well, explicitly state that no significant
violations were found.
