(A) add external import

(B) add trait
(C) add enum
    (C) basic c enum {cm}
    (D) enum with expression

(C) add match
    (C) add numaric switch like match {cm}
    (D) add string switch like match
    (D) add array switch like match
    (E) add type matching

(C) add union

(A) merge ast:Expression Array and Arracontructor using AnyArray

# Bugs
- `{ Io.Println("test") }` (unexpected `}` error)
- `[@]char methode() {}` (`[@]char` the `[@]` is unexpected)
- `call() as *char`
- `return *ptr` (generates incorrect llvm code)
- `if condition {}` (`if condition == true {}` does work only if right is literal)
- `Fn(): int {if true {return 1} else {return 2}}` 
- `Fn() {innerFn() {}}` (if in non crate mod innerFn gets placed in crate mod)