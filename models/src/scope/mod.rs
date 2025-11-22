//! Scope management for the Soul language.
//!
//! This module provides functionality for managing lexical scopes, including
//! tracking variables, functions, and types within different scopes.

pub mod scope;
pub mod scope_builder;

#[cfg(test)]
mod test_scope;