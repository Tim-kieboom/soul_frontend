use hir_model::{HirTree};
use soul_typed_context::{TypedContext};
use soul_utils::{error::SoulError, sementic_level::SementicFault, vec_map::VecMap};
use thir_model::{self as thir, ThirTree};

mod lower_function;
mod lower_item;

pub fn lower_to_thir(hir: &HirTree, typed: &TypedContext) -> thir::ThirResponse {
    let mut context = ThirContext::new(hir, typed);

    for item in hir.root.items.values() {
        context.lower_item(item);
    }

    thir::ThirResponse {
        tree: context.thir,
        faults: context.faults,
    }
}

struct ThirContext<'a> {
    hir: &'a HirTree,
    typed_context: &'a TypedContext,
    id_generator: thir::IdGenerator,
    item_id_generator: thir::ItemIdGenerator,

    thir: ThirTree,
    faults: Vec<SementicFault>,
}
impl<'a> ThirContext<'a> {
    pub(crate) fn new(hir: &'a HirTree, typed: &'a TypedContext) -> Self {
        let thir = ThirTree {
            items: VecMap::new(),
            global_expressions: VecMap::new(),
        };
        Self {
            hir,
            thir,
            typed_context: typed,
            faults: vec![],
            item_id_generator: thir::ItemIdGenerator::new(),
            id_generator: thir::IdGenerator::new(),
        }
    }

    pub(crate) fn log_error(&mut self, err: SoulError) {
        self.faults.push(SementicFault::error(err));
    }
}
