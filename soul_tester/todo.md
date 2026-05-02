(A) add external import

(B) add trait
(C) add enum
    (C) basic c enum
    (D) enum with expression

(C) add match
(C) add union

# Bugs
- `[@]char methode() {}`
- `call() as *char`
- `return *ptr`
- `if condition {}` (`if condition == true {}` does work only if right is literal)
- `if !condition {}`
- `Fn(): int {if true {return 1} else {return 2}}`
- `Fn() {innerFn() {}}` (if in non crate mod innerFn gets placed in crate mod)