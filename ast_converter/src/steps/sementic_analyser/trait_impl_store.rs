use std::collections::HashMap;

use soul_ast::{
    abstract_syntax_tree::soul_type::{SoulType, TypeKind},
    error::{SoulError, SoulErrorKind, SoulResult},
    sementic_models::scope::NodeId,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TraitImplEntry {
    trait_type: SoulType,
    of_type: SoulType,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TraitImplStore {
    store: HashMap<NodeId, Vec<TraitImplEntry>>,
}
impl TraitImplStore {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }

    pub fn insert(&mut self, trait_type: SoulType, of_type: SoulType) -> SoulResult<()> {
        let node_id = match trait_type.kind {
            TypeKind::Trait(val) => val,
            other => {
                return Err(SoulError::new(
                    format!("trait_type is not trait but {}", other.display_variant()),
                    SoulErrorKind::InvalidTypeKind,
                    Some(trait_type.span),
                ));
            }
        };

        self.store.entry(node_id).or_default().push(TraitImplEntry {
            trait_type,
            of_type,
        });

        Ok(())
    }
}
