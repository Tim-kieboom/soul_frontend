use ast::{Block, Function, Statement, StatementKind, VarTypeKind, scope::{ScopeValueKind}};

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(super) fn collect_block(&mut self, block: &mut Block) {
        self.push_scope(&mut block.scope_id);
        self.collect_scopeless_block(block);
        self.pop_scope();
    }

    pub(crate) fn collect_scopeless_block(&mut self, block: &mut Block) {
        block.node_id = Some(self.alloc_id());
        for statement in &mut block.statements {
            self.collect_statement(statement);    
        }
    }

    fn collect_statement(&mut self, statement: &mut Statement) {
         match &mut statement.node {
            StatementKind::Import(_) => todo!("impl import trait collection"),
            StatementKind::Variable(variable) => {
                let id = self.declare_value(ScopeValueKind::Variable(variable));
                self.store.insert_variable_type(id, variable.ty.clone());
                
                match &mut variable.ty {
                    VarTypeKind::NonInveredType(soul_type) => self.collect_type(soul_type),
                    VarTypeKind::InveredType(_) => (),
                }

                if let Some(value) = &mut variable.initialize_value {
                    self.collect_expression(value);
                }
            }
            StatementKind::Function(function) => {
                self.collect_function(function);
            }
            StatementKind::Expression{id, expression, ends_semicolon:_} => {
                *id = Some(self.alloc_id());
                self.collect_expression(expression);
            }
            StatementKind::Assignment(assignment) => {
                assignment.node_id = Some(self.alloc_id());
                self.collect_expression(&mut assignment.left);
                self.collect_expression(&mut assignment.right);
            }

        }
    }

    fn collect_function(&mut self, function: &mut Function) {
        let id = self.declare_value(ScopeValueKind::Function(function));
        let prev = self.current_function;
        self.current_function = Some(id);
        
        let signature = &mut function.signature.node;
        self.collect_type(&mut signature.methode_type);
        self.collect_type(&mut signature.return_type);

        self.push_scope(&mut function.block.scope_id);
        self.declare_parameters(&mut function.signature.node.parameters);
        self.collect_scopeless_block(&mut function.block);
        self.pop_scope();

        self.store.insert_functions(id, function.signature.node.clone());
        self.current_function = prev;
    }
}