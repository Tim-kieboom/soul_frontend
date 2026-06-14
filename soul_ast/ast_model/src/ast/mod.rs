use soul_utils::impl_soul_ids;

pub mod block;
pub mod literal;
pub mod soul_type;
pub mod operators;
pub mod statements;
pub mod expression;

impl_soul_ids!(NodeId);
