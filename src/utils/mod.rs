//! Shared helpers for stylesheet lint rules.
//!
//! Use [`ast`] for parsed nodes, [`scan`] for raw source, and [`syntax`] for lexical
//! checks. [`text`] handles string manipulation, [`options`] reads rule configuration,
//! and [`mod@reference`] supplies CSS names and lookup tables. All six modules' public
//! items are also re-exported here.

#![allow(dead_code, unused_imports)]
#![deny(missing_docs)]

pub mod ast;
pub mod options;
pub mod reference;
pub mod scan;
pub mod syntax;
pub mod text;

pub use ast::*;
pub use options::*;
pub use reference::*;
pub use scan::*;
pub use syntax::*;
pub use text::*;
