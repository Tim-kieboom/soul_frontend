(A) add external import

(B) add trait
(C) add enum
    (C) basic c enum {cm}
    (D) enum with expression

(D) add union struct  

(D) add match
    (D) variable match
    (D) if match
    (D) tuple match
    
(E) make borrow checker
(F) make coroutines

# Bugs
- `__clib_Duration_now(&this.start)` (this.start does not work)
- `const buffer: [64]` (no error is thrown)
- `{ Io.Println("test") }` (unexpected `}` error)
- `[@]char methode() {}` (`[@]char` the `[@]` is unexpected)
- `call() as *char`
- `return *ptr` (generates incorrect llvm code)
- `if condition {}` (`if condition == true {}` does work only if right is literal)
- `Fn(): int {if true {return 1} else {return 2}}` 
- `Fn() {innerFn() {}}` (if in non crate mod innerFn gets placed in crate mod)