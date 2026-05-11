//! Pest grammar integration
//!
//! This module provides the Pest parser generated from `grammar/query.pest`.

use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar/query.pest"]
pub struct QueryGrammar;
