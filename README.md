# Soul

A memory-safe, zero-cost abstract systems programming language that compiles to LLVM.

[![Rust](https://img.shields.io/badge/Rust-1.75+-dea058.svg)](https://www.rust-lang.org)
[![LLVM](https://img.shields.io/badge/LLVM-16+-5C6BC0.svg)](https://llvm.org)

## Overview

Soul is a systems programming language designed for high-performance, memory-safe applications. It features zero-cost abstractions, trait-based polymorphism, and generics with type inference, targeting efficient native code via LLVM.

## Features

- **Memory Safety** - No garbage collection, lifetime-based memory management
- **Zero-Cost Abstractions** - High-level features that compile to efficient native code
- **Generics** - Generic functions and types with full type inference
- **Traits** - Trait-based polymorphism with trait bounds (`impl` blocks)
- **Result-Based Error Handling** - `Res<V, E>` type for explicit error handling
- **Method Syntax** - Multiple calling conventions: `&this`, `@this`, `this`, consume
- **Comptime** - Compile-time constant evaluation (`const` and `literal`)
- **FFI** - Seamless C interop via `extern "C"`
- **Control Flow** - `if`/`elif`/`else`, `while`, `for` loops, `match`

## Example

```soul
struct Duration {
    mut sec: u64
    mut nano: u32
}

use Duration {
    Empty(): Duration {
        Duration{sec: 0, nano: 0}
    }

    Now(): Duration {
        extern "C" __clib_Duration_now(duration: &Duration)
        mut time = Duration.Empty()
        __clib_Duration_now(&time)
        time
    }

    Print(@this) {
        sec := this.sec
        nano := this.nano
        print("Duration{sec: ")
        print_uint(sec as uint)
        print(", nano: ")
        print_uint(nano as uint)
        print("}")
    }
}

main() {
    println("hello world")
    now := Duration.Now()
    now.Print()
}
```

## Building

### Prerequisites

- Rust 1.75+
- LLVM 16 (via inkwell)
- A C compiler (for stdlib)

### Build

```bash
# Clone the repository
git clone https://github.com/Tim-kieboom/soul_frontend.git
cd soul_frontend

# Build all crates
cargo build --release

# Run the tester (compiles and runs main.soul)
cargo run -p soul_tester
```

## Project Structure

```
soul_frontend/
├── soul_utils/         # Common utilities (IDs, spans, errors, maps)
├── soul_tokenizer/    # Lexical analysis
├── soul_ast/          
│   ├── ast_model/     # AST data structures
│   ├── ast_parser/    # Parser
│   ├── soul_name_resolver/  # Name resolution
│   └── run_ast/       # AST orchestration
├── soul_hir/
│   ├── hir_model/     # HIR structures
│   ├── hir_parser/    # AST -> HIR
│   ├── typed_hir_model/    # Typed HIR
│   ├── typed_hir_parser/    # Type inference
│   ├── hir_literal_interpreter/  # Compile-time evaluation
│   └── run_hir/      # HIR orchestration
├── soul_mir/
│   ├── mir_parser/    # HIR -> MIR
│   └── run_mir/      # MIR orchestration
├── soul_ir/         # LLVM IR generation
└── soul_tester/     # Test harness
```

## Pipeline

Soul uses a multi-phase compilation pipeline:

```
source → tokenizer → AST → name resolution → HIR → typed HIR → MIR → LLVM IR → binary
```

1. **Tokenization** - Source text → Token stream
2. **Parsing** - Token stream → AST
3. **Name Resolution** - Resolves identifiers, collects declarations
4. **HIR Lowering** - AST → HIR (High-level IR)
5. **Type Inference** - Type checking and inference
6. **MIR Lowering** - HIR → MIR (Mid-level IR, SSA-like)
7. **LLVM Generation** - MIR → LLVM IR via inkwell
8. **Code Generation** - LLVM → native binary

## Language Syntax

### Functions

```soul
// Static function
const add(a: int, b: int): int {
    a + b
}

// Method with mutable reference
int addMut(&this, b: int): int {
    a := *this
    a + b
}

// Method with const reference
int addRef(@this, b: int): int {
    a := *this
    a + b
}

// Method consuming self
int add(this, b: int): int {
    this + b
}

// Generic function
gen<T>(value: T): T {
    value
}
```

### Structs & Use Blocks

```soul
struct Point {
    x: int
    y: int
}

use Point {
    // Constructor
    New(x: int, y: int): Point {
        Point{x, y}
    }

    // Method
    distance(@this): int {
        this.x + this.y
    }
}
```

### Error Handling

```soul
union Res<V, E> {
    Ok(V),
    Err(E),
}

// Usage
tryPush(value: T): Res {
    if full() {
        return Err(ErrFull{})
    }
    Ok(())
}
```

### Control Flow

```soul
// If expression
val := if condition {
    1
} else {
    2
}

// While loop
while condition {
    counter = counter + 1
}

// For loop
for item in list {
    process(item)
}

// Match
match result {
    Ok(value) => handle(value),
    Err(err) => handleError(err),
}
```

### Arrays

```soul
// Stack array (fixed size)
arr: [10]int

// Heap array (dynamic)
buffer: [*]int

// Slice (reference to array)
slice: [&]int

// Const slice
cslice: [@]int
```

## Status

Soul is in active development. The core compiler pipeline is functional, including:
- ✅ Lexer and parser
- ✅ Name resolution
- ✅ HIR lowering
- ✅ Type inference with generics
- ✅ MIR generation
- ✅ LLVM code generation

### In Progress

- ⚙️ Traits and trait bounds
- ⚙️ Enum definitions
- ⚙️ Union pattern matching