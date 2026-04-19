(A) add import
    (A) Parser {cm}
        (A) change import parser - parse dot-style paths (std.Fmt) {cm}
            - Remove StringLiteral/TokenKind::Ident path handling {cm}
            - Add dot-separated path parsing (std.Fmt.{A, B}) {cm}
            - Parse module path as Identifier chain (std.Fmt) {cm}
        (D) impl multi import `import (crate.fmt\n std.io)` {cm}
        (D) impl as of item import `import crate.fmt.{Println as newPrint}` {cm}

    (B) add module node to AST {cm}
        (A) add StatementKind::Module(Module) in ast_model/statement.rs {cm} 
        (B) add module parser in ast_parser {cm}

    (D) HIR changes
        (A) update hir_parser to handle new import format {cm}
        (B) update typed_hir if needed for module resolution {cm}
        (C) update mir if needed
        (D) update llvm code

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
- if statements with fn with return type
