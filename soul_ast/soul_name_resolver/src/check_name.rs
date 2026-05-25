use soul_utils::{
    Ident,
    error::{SoulError, SoulErrorKind, SoulResult},
    print_breakpoint, soul_error_internal,
};

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(crate) fn check_variable_name(&mut self, name: &Ident) {
        if let Err(err) = self.inner_check_variable_name(name) {
            self.log_error(err);
        }
    }

    pub(crate) fn check_function_name(&mut self, name: &Ident) {
        if let Err(err) = self.inner_check_function_name(name) {
            self.log_error(err);
        }

        if name.as_str() == "match_int" {
            print_breakpoint!()
        }

        let Some(id) = self.current.function else {
            return;
        };

        let Some((function, _)) = self.store.get_function(id) else {
            self.log_error(soul_error_internal!(
                format!("parent function {id:?} not found"),
                Some(name.span)
            ));
            return;
        };

        if function.name.as_str() == name.as_str() {
            self.log_error(SoulError::new(
                "parent and child function can not have the same name",
                SoulErrorKind::InvalidFunctionName,
                Some(name.span),
            ));
        }
    }

    fn inner_check_variable_name(&mut self, name: &Ident) -> SoulResult<()> {
        let mut chars = name.as_str().chars();
        let first = chars.next().ok_or(SoulError::new(
            "variable name can not be empty",
            SoulErrorKind::InvalidVariableName,
            Some(name.span),
        ))?;

        if !first.is_alphabetic() && first != '_' {
            return Err(SoulError::new(
                format!("variable name should not start with '{first}' (start with letter or '_')"),
                SoulErrorKind::InvalidVariableName,
                Some(name.span),
            ));
        }

        Ok(())
    }

    fn inner_check_function_name(&mut self, name: &Ident) -> SoulResult<()> {
        let mut chars = name.as_str().chars();
        let first = chars.next().ok_or(SoulError::new(
            "function name can not be empty",
            SoulErrorKind::InvalidFunctionName,
            Some(name.span),
        ))?;

        if !first.is_alphabetic() && first != '_' {
            return Err(SoulError::new(
                format!("function name should not start with '{first}' (start with letter or '_')"),
                SoulErrorKind::InvalidFunctionName,
                Some(name.span),
            ));
        }

        if name.as_str().contains("___") {
            return Err(SoulError::new(
                "function name should not have '___' in the name",
                SoulErrorKind::InvalidFunctionName,
                Some(name.span),
            ));
        }

        Ok(())
    }
}
