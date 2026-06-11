# Compiler TODO

# Phase 2: Lexer

## Tokens

* [ ] Identifiers
* [ ] Keywords
* [ ] Integer literals
* [ ] Float literals
* [ ] String literals
* [ ] Character literals
* [ ] Comments
* [ ] Operators
* [ ] Delimiters

## Diagnostics

* [ ] Invalid characters
* [ ] Unterminated strings
* [ ] Unterminated comments

## Tests

* [ ] Tokenization tests
* [ ] Error tests

---

# Phase 3: Parser

## Expressions

* [ ] Literals
* [ ] Variable references
* [ ] Unary expressions
* [ ] Binary expressions
* [ ] Function calls
* [ ] Field access
* [ ] Index access
* [ ] Assignment

## Statements

* [ ] Variable declarations
* [ ] Return statements
* [ ] If statements
* [ ] While loops
* [ ] For loops
* [ ] Block statements

## Declarations

* [ ] Functions
* [ ] Structs
* [ ] Enums
* [ ] Modules
* [ ] Constants

## Tests

* [ ] AST snapshot tests
* [ ] Syntax error tests

---

# Phase 4: AST

## Core Nodes

* [ ] Expression hierarchy
* [ ] Statement hierarchy
* [ ] Declaration hierarchy
* [ ] Type nodes

## Utilities

* [ ] AST printer
* [ ] AST visitor
* [ ] AST dumper

---

# Phase 5: Name Resolution

## Scopes

* [ ] Global scope
* [ ] Module scope
* [ ] Function scope
* [ ] Block scope

## Symbol Tables

* [ ] Variables
* [ ] Functions
* [ ] Structs
* [ ] Enums
* [ ] Constants

## Diagnostics

* [ ] Undefined symbols
* [ ] Duplicate symbols
* [ ] Shadowing warnings

---

# Phase 6: Type System

## Primitive Types

* [ ] bool
* [ ] integers
* [ ] floats
* [ ] char
* [ ] string

## Compound Types

* [ ] arrays
* [ ] slices
* [ ] pointers
* [ ] references
* [ ] function types

## User Types

* [ ] structs
* [ ] enums

## Type Checking

* [ ] assignments
* [ ] function calls
* [ ] returns
* [ ] operators
* [ ] pattern matching

## Diagnostics

* [ ] type mismatch
* [ ] invalid casts
* [ ] missing returns

---

# Phase 7: HIR

## Lower AST → HIR

* [ ] Simplify syntax sugar
* [ ] Resolve symbols
* [ ] Attach types

## Validation

* [ ] HIR printer
* [ ] HIR tests

---

# Phase 8: MIR

## Control Flow

* [ ] Basic blocks
* [ ] Jumps
* [ ] Branches
* [ ] Returns

## Instructions

* [ ] Load
* [ ] Store
* [ ] Call
* [ ] Move
* [ ] Borrow
* [ ] Drop

## Validation

* [ ] MIR printer
* [ ] MIR tests

---

# Phase 9: Ownership System

## Move Semantics

* [ ] Ownership tracking
* [ ] Move operations
* [ ] Copy operations

## Diagnostics

* [ ] Use after move
* [ ] Double move
* [ ] Invalid move

## Tests

* [ ] Move checker tests

---

# Phase 10: Borrow Checker

## Borrow Types

* [ ] Shared borrow (&T)
* [ ] Mutable borrow (&mut T)

## Rules

* [ ] Multiple shared borrows
* [ ] Single mutable borrow
* [ ] Shared vs mutable conflicts

## Diagnostics

* [ ] Borrow conflicts
* [ ] Mutable aliasing
* [ ] Borrowed value modification

## Tests

* [ ] Borrow checker tests

---

# Phase 11: Lifetime Analysis

## Regions

* [ ] Lifetime creation
* [ ] Lifetime propagation
* [ ] Lifetime constraints

## Diagnostics

* [ ] Dangling references
* [ ] Escaping references
* [ ] Invalid returns

## Tests

* [ ] Lifetime tests

---

# Phase 12: Standard Library

## Core

* [ ] Option
* [ ] Result
* [ ] String
* [ ] Vec
* [ ] Slice

## Collections

* [ ] HashMap
* [ ] HashSet

## Utilities

* [ ] Iterator API

---

# Phase 13: Backend

## C Backend

* [ ] Type generation
* [ ] Function generation
* [ ] Struct generation
* [ ] Runtime generation

## Output

* [ ] Single C file output
* [ ] Multi-file output

---

# Phase 14: Tooling

## CLI

* [ ] Build command
* [ ] Run command
* [ ] Test command

## Formatting

* [ ] Formatter

## LSP

* [ ] Go to definition
* [ ] Hover info
* [ ] Diagnostics

---

# Phase 15: Optimization

## MIR Optimizations

* [ ] Constant folding
* [ ] Dead code elimination
* [ ] Copy propagation

## Backend Optimizations

* [ ] Inline small functions
* [ ] Remove redundant drops

---

# Phase 16: Self Hosting

* [ ] Compiler written in language
* [ ] Standard library rewritten
* [ ] Bootstrap compiler
* [ ] Self-hosted release
