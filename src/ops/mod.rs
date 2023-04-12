pub mod controlflow;
pub mod custom;
pub mod function;
pub mod leaf;
pub mod module;

use crate::types::{Signature, SignatureDescription};

pub use controlflow::ControlFlowOp;
pub use custom::{CustomOp, OpDef, OpaqueOp};
pub use function::FunctionOp;
pub use leaf::LeafOp;
pub use module::{ConstValue, ModuleOp};
use smol_str::SmolStr;

/// A generic node operation
pub trait Op {
    /// The name of the operation.
    fn name(&self) -> SmolStr;
    /// The description of the operation.
    fn description(&self) -> &str {
        ""
    }
    /// The signature of the operation.
    ///
    /// TODO: Return a reference? It'll need some lazy_statics to make it work.
    fn signature(&self) -> Signature;
    /// Optional description of the ports in the signature.
    ///
    /// TODO: Implement where possible
    fn signature_desc(&self) -> Option<SignatureDescription> {
        None
    }
}

/// The concrete operation types for a node in the HUGR.
///
/// TODO: Flatten the enum? It improves efficiency, but makes it harder to read.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum OpType {
    /// A module region node.
    Module(ModuleOp),
    /// A control flow node
    ControlFlow(ControlFlowOp),
    /// A function manipulation node
    Function(FunctionOp),
}

impl Op for OpType {
    fn name(&self) -> SmolStr {
        match self {
            OpType::Module(op) => op.name(),
            OpType::ControlFlow(op) => op.name(),
            OpType::Function(op) => op.name(),
        }
    }

    fn signature(&self) -> Signature {
        match self {
            OpType::Module(op) => op.signature(),
            OpType::ControlFlow(op) => op.signature(),
            OpType::Function(op) => op.signature(),
        }
    }
}

impl Default for OpType {
    fn default() -> Self {
        Self::Function(Default::default())
    }
}

impl From<ModuleOp> for OpType {
    fn from(op: ModuleOp) -> Self {
        Self::Module(op)
    }
}

impl From<ControlFlowOp> for OpType {
    fn from(op: ControlFlowOp) -> Self {
        Self::ControlFlow(op)
    }
}

impl<T> From<T> for OpType
where
    T: Into<FunctionOp>,
{
    fn from(op: T) -> Self {
        Self::Function(op.into())
    }
}
