use ast::{Match, MatchPattern};
use hir::{ExpressionId, MatchArm as MatchArmHir, MatchPatternHir};
use soul_utils::{error::SoulError, error::SoulErrorKind, span::Span};

use crate::HirContext;

impl<'a> HirContext<'a> {
    pub(super) fn lower_match(
        &mut self,
        id: ExpressionId,
        ast_match: &Match,
        span: Span,
    ) -> hir::Expression {
        let scrutinee = self.lower_expression(&ast_match.scrutinee);

        fn lower_pattern(pat: &MatchPattern) -> MatchPatternHir {
            match pat {
                MatchPattern::Literal(lit) => MatchPatternHir::Literal(lit.clone()),
                MatchPattern::Wildcard => MatchPatternHir::Wildcard,
                MatchPattern::Array(elements) => {
                    MatchPatternHir::Array(elements.iter().map(lower_pattern).collect())
                }
                // Constructor and Binding handled outside this helper
                _ => unreachable!(),
            }
        }

        let mut arms = Vec::new();
        for arm in &ast_match.arms {
            let pattern = match &arm.pattern {
                MatchPattern::Binding { ident, id: node_id } => {
                    let local = self.id_generator.alloc_local();
                    let ty = self.new_infer_type(vec![], None, ident.span);
                    self.insert_variable(ident, local, ty, None);
                    if let Some(node_id) = node_id {
                        self.node_id_to_local.insert(*node_id, local);
                    }
                    MatchPatternHir::Binding(local)
                }
                MatchPattern::Constructor {
                    type_name,
                    variant_name,
                    binding,
                    binding_id,
                } => {
                    let union_id = match self.lookup_union(type_name) {
                        Some(id) => id,
                        None => {
                            self.log_error(SoulError::new(
                                format!("'{}' is not a union type", type_name.as_str()),
                                SoulErrorKind::InvalidIdent,
                                Some(span),
                            ));
                            let body = self.lower_block(&arm.body);
                            arms.push(MatchArmHir {
                                pattern: MatchPatternHir::Wildcard,
                                body,
                            });
                            continue;
                        }
                    };

                    let union = match self.tree.info.types.id_to_union(union_id) {
                        Some(val) => val,
                        None => {
                            let body = self.lower_block(&arm.body);
                            arms.push(MatchArmHir {
                                pattern: MatchPatternHir::Wildcard,
                                body,
                            });
                            continue;
                        }
                    };

                    let variant_index = match union
                        .variants
                        .iter()
                        .position(|v| v.name.as_str() == variant_name.as_str())
                    {
                        Some(idx) => idx,
                        None => {
                            self.log_error(SoulError::new(
                                format!(
                                    "'{}' is not a variant of '{}'",
                                    variant_name.as_str(),
                                    type_name.as_str()
                                ),
                                SoulErrorKind::InvalidIdent,
                                Some(span),
                            ));
                            let body = self.lower_block(&arm.body);
                            arms.push(MatchArmHir {
                                pattern: MatchPatternHir::Wildcard,
                                body,
                            });
                            continue;
                        }
                    };

                    let binding_local = binding.as_ref().map(|binding_ident| {
                        let local = self.id_generator.alloc_local();
                        let ty = self.new_infer_type(vec![], None, binding_ident.span);
                        self.insert_variable(binding_ident, local, ty, None);
                        if let Some(node_id) = binding_id {
                            self.node_id_to_local.insert(*node_id, local);
                        }
                        local
                    });

                    MatchPatternHir::Constructor {
                        union_id,
                        variant_index,
                        binding: binding_local,
                    }
                }
                _ => lower_pattern(&arm.pattern),
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
