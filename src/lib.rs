#![warn(missing_docs)]

//! `hugr` is the Hierarchical Unified Graph Representation of quantum circuits
//! and operations in the Quantinuum ecosystem.
//!
//! # Features
//!
//! - `serde` enables serialization and deserialization of the components and
//!   structures.
//!

pub mod algorithm;
pub mod builder;
pub mod convex;
pub mod extensions;
pub mod hugr;
pub mod macros;
pub mod ops;
#[cfg(feature = "patternmatching")]
pub mod pattern;
pub mod replacement;
pub mod resource;
pub mod rewrite;
pub mod types;
mod utils;

pub use crate::hugr::{Direction, Hugr, Node, Port, Wire};
pub use crate::replacement::SimpleReplacement;
pub use crate::resource::Resource;
pub use crate::rewrite::{Rewrite, RewriteError};
