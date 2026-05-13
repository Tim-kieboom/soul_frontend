use ast::{Match, MatchPattern};
use hir::{ExpressionId, MatchArm as MatchArmHir, MatchPatternHir};
use soul_utils::span::Span;

use crate::HirContext;

impl<'a> HirContext<'a> {
    pub(super) fn lower_match(
        &mut self,
        id: ExpressionId,
        ast_match: &Match,
        span: Span,
    ) -> hir::Expression {
        let scrutinee = self.lower_expression(&ast_match.scrutinee);

        let mut arms = Vec::new();
        for arm in &ast_match.arms {
            let pattern = match &arm.pattern {
                MatchPattern::Literal(lit) => MatchPatternHir::Literal(lit.clone()),
                MatchPattern::Wildcard => MatchPatternHir::Wildcard,
            };
            let body = self.lower_block(&arm.body);
            arms.push(MatchArmHir { pattern, body });
        }

        hir::Expression {
            id,
            ty: self.new_infer_type(vec![], None, span),
            kind: hir::ExpressionKind::Match { scrutinee, arms },
        }
    }
}
