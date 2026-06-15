use soul_utils::impl_soul_ids;

pub mod block;
pub mod expression;
pub mod literal;
pub mod operators;
pub mod soul_type;
pub mod statements;

impl_soul_ids!(NodeId);
