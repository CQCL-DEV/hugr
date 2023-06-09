//! Module-level operations

use smol_str::SmolStr;

use crate::types::{ClassicType, EdgeKind, Signature, SimpleType};

use super::{impl_op_name, tag::OpTag, OpTrait};

/// The root of a module, parent of all other `OpType`s.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Module;

impl_op_name!(Module);

impl OpTrait for Module {
    fn description(&self) -> &str {
        "The root of a module, parent of all other `OpType`s"
    }

    fn tag(&self) -> super::tag::OpTag {
        OpTag::ModuleRoot
    }
}

/// A function definition.
///
/// Children nodes are the body of the definition.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Def {
    /// Name of function
    pub name: String,
    /// Signature of the function
    pub signature: Signature,
}

impl_op_name!(Def);
impl OpTrait for Def {
    fn description(&self) -> &str {
        "A function definition"
    }

    fn tag(&self) -> OpTag {
        OpTag::Def
    }

    fn other_outputs(&self) -> Option<EdgeKind> {
        Some(EdgeKind::Const(ClassicType::graph_from_sig(
            self.signature.clone(),
        )))
    }
}

/// External function declaration, linked at runtime.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Declare {
    /// Name of function
    pub name: String,
    /// Signature of the function
    pub signature: Signature,
}

impl_op_name!(Declare);

impl OpTrait for Declare {
    fn description(&self) -> &str {
        "External function declaration, linked at runtime"
    }

    fn tag(&self) -> OpTag {
        OpTag::Function
    }

    fn other_outputs(&self) -> Option<EdgeKind> {
        Some(EdgeKind::Const(ClassicType::graph_from_sig(
            self.signature.clone(),
        )))
    }
}

/// A type alias definition, used only for debug/metadata.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AliasDef {
    /// Alias name
    pub name: SmolStr,
    /// Aliased type
    pub definition: SimpleType,
}
impl_op_name!(AliasDef);
impl OpTrait for AliasDef {
    fn description(&self) -> &str {
        "A type alias definition"
    }

    fn tag(&self) -> OpTag {
        OpTag::Alias
    }
}

/// A type alias declaration. Resolved at link time.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AliasDeclare {
    /// Alias name
    pub name: SmolStr,
    /// Flag to signify type is linear
    pub linear: bool,
}

impl_op_name!(AliasDeclare);

impl OpTrait for AliasDeclare {
    fn description(&self) -> &str {
        "A type alias declaration"
    }

    fn tag(&self) -> OpTag {
        OpTag::Alias
    }
}
