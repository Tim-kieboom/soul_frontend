(A) add import
    (A) Parser
        (A) change import parser - parse dot-style paths (std.Fmt) {cm}
            - Remove StringLiteral/TokenKind::Ident path handling {cm}
            - Add dot-separated path parsing (std.Fmt.{A, B}) {cm}
            - Parse module path as Identifier chain (std.Fmt) {cm}
        (D) impl multi import `import (crate.fmt\n std.io)` {cm}
        (D) impl as of item import `import crate.fmt.{Println as newPrint}`

    (B) add module node to AST
        (A) add StatementKind::Module(Module) in ast_model/statement.rs {cm} 
        (B) add module parser in ast_parser {cm}
        (C) collect module declarations in name_resolver
            - Track module name in scope, handle pub/private visibility

    (C) name resolution changes
        (A) change import resolution to use new ImportPath format
            - soul_name_resolver/src/collect/collect_statement.rs
            - Update collect_import_path to handle dot-style module paths
        (B) track pub vs mod (private) visibility in module scope
            - Capital-starting names = pub, lowercase = private
            - Filter exports based on import kind (Items, All, This)
        (C) update resolve_import_functions
            - Remove function name mangling (format!("{}::{}"))
            - Keep simple function name if importing specific items
            - Handle module path resolution (std.Fmt -> crate std/Fmt.soul)

    (D) HIR changes
        (A) update hir_parser to handle new import format
            - soul_hir/hir_parser/src/statement/mod.rs
        (B) update typed_hir if needed for module resolution
        (C) update mir if needed

    (E) test and verify
        (A) create test files with new import style
        (B) run existing tests
        (C) run lint/typecheck

(B) add trait
(C) add union
(C) add enum