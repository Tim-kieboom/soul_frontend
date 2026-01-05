use crate::HirLowerer;
use soul_hir::{self as hir, HirId};
use soul_ast::abstract_syntax_tree::{self as ast, ForPattern, Ident, SoulType, Visibility};
use soul_utils::Span;

enum ForPatternResult {
    Binding((Ident, hir::Binding), Span),
    Variable(hir::Variable, Span),
}

impl<'hir> HirLowerer<'hir> {
    pub(super) fn lower_for_pattern(&mut self, for_pattern: &Option<ForPattern>) -> Option<Option<HirId>> {
        
        let pattern = match for_pattern {
            Some(val) => val,
            None => return Some(None),
        };

        let (kind, span) = match self.get_hir_for_pattern(pattern, 0.to_string(), None)? {
            ForPatternResult::Binding((_, binding), span) => {
                (hir::StatementKind::Binding(binding), span)
            }
            ForPatternResult::Variable(variable, span) => {
                (hir::StatementKind::Variable(Box::new(variable)), span)
            }
        };

        let id = match self.add_statement(hir::Statement::new(kind, span)) {
            Ok(val) => val,
            Err(err) => {
                self.log_error(err);
                return None
            }
        };

        Some(Some(
            id
        ))
    }

    fn get_hir_for_pattern(&mut self, for_pattern: &ForPattern, name: String, span: Option<Span>) -> Option<ForPatternResult> {
        Some(match &for_pattern {
            ast::ForPattern::Ident { ident, .. } => {
                ForPatternResult::Variable(hir::Variable {
                    id: self.alloc_id(),
                    ty: self.get_hir_type(&SoulType::none(ident.span))?,
                    name: ident.clone(),
                    vis: Visibility::Private,
                    value: None,
                }, ident.span)
            }
            ast::ForPattern::Tuple(tuple) => {
                self.get_tuple(tuple, name, span)?
            }
            ast::ForPattern::NamedTuple(named_tuple) => {
                self.get_named_tuple(named_tuple, name, span)?
            }
        })
    }

    fn get_named_tuple(&mut self, named_tuple: &Vec<(Ident, ForPattern)>, name: String, span: Option<Span>) -> Option<ForPatternResult> {
        let mut my_binding = hir::Binding{ id: self.alloc_id(), ty: None, variables: Vec::with_capacity(named_tuple.len()) };

        let (ident, pattern) = named_tuple.first()?;
        let mut my_span = match self.get_hir_for_pattern(pattern, ident.node.clone(), Some(ident.span))? {
            ForPatternResult::Binding(_, span) => span,
            ForPatternResult::Variable(_, span) => span,
        };
        
        
        for (Ident{ node, span, ..}, pattern) in named_tuple.iter() {
            let name = node.clone();
            let span = Some(*span);

            match self.get_hir_for_pattern(pattern, name, span)? {
                ForPatternResult::Binding((name, binding), span) => {
                    let id = self.resolve_binding_result(binding, span);
                    my_binding.variables.push((name, id));
                    my_span = my_span.combine(span)
                }
                ForPatternResult::Variable(variable, span) => {
                    let name = variable.name.clone();
                    let id = self.resolve_variable_result(variable, span)?;
                    my_binding.variables.push((name, id));
                    my_span = my_span.combine(span)
                }
            }
        }

        let span = match span {
            Some(val) => val,
            None => my_span,
        };
        Some(ForPatternResult::Binding(
            (Ident::new(name, span), my_binding), my_span
        ))
    }

    fn get_tuple(&mut self, tuple: &Vec<ForPattern>, name: String, span: Option<Span>) -> Option<ForPatternResult> {
        let mut my_binding = hir::Binding{ id: self.alloc_id(), ty: None, variables: Vec::with_capacity(tuple.len()) };

        let mut my_span = match self.get_hir_for_pattern(tuple.first()?, 0.to_string(), None)? {
            ForPatternResult::Binding(_, span) => span,
            ForPatternResult::Variable(_, span) => span,
        };
        
        
        for (i, el) in tuple.iter().enumerate() {
            match self.get_hir_for_pattern(el, i.to_string(), None)? {
                ForPatternResult::Binding((name, binding), span) => {
                    let id = self.resolve_binding_result(binding, span);
                    my_binding.variables.push((name, id));
                    my_span = my_span.combine(span)
                }
                ForPatternResult::Variable(variable, span) => {
                    let name = variable.name.clone();
                    let id = self.resolve_variable_result(variable, span)?;
                    my_binding.variables.push((name, id));
                    my_span = my_span.combine(span)
                }
            }
        }

        let span = match span {
            Some(val) => val,
            None => my_span,
        };
        Some(ForPatternResult::Binding(
            (Ident::new(name, span), my_binding), my_span
        ))
    }

    fn resolve_binding_result(&mut self, binding: hir::Binding, span: Span) -> HirId {
        let kind = hir::ExpressionKind::Tuple(binding.variables);
        self.add_expression(hir::Expression::new(kind, span))
    }

    fn resolve_variable_result(&mut self, variable: hir::Variable, span: Span) -> Option<HirId> {
        let kind = hir::StatementKind::Variable(Box::new(variable));
        match self.add_statement(hir::Statement::new(kind, span)) {
            Ok(val) => Some(val),
            Err(err) => {
                self.log_error(err);
                None
            }
        }
    }
}