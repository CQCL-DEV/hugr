//! Handles to nodes in HUGR, to be used during building phase.
//!
use crate::types::{ClassicType, SimpleType};

use super::Wire;
use core::iter::FusedIterator;
use derive_more::From as DerFrom;
use itertools::Itertools;
use portgraph::NodeIndex;
use smol_str::SmolStr;

#[derive(Debug, Clone)]
/// Iterator over output wires of a [`BuildHandle`].
pub struct Outputs {
    node: NodeIndex,
    range: std::ops::Range<usize>,
}

impl Iterator for Outputs {
    type Item = Wire;

    fn next(&mut self) -> Option<Self::Item> {
        self.range.next().map(|offset| Wire(self.node, offset))
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.range.nth(n).map(|offset| Wire(self.node, offset))
    }

    #[inline]
    fn count(self) -> usize {
        self.range.count()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
}

impl ExactSizeIterator for Outputs {
    #[inline]
    fn len(&self) -> usize {
        self.range.len()
    }
}

impl DoubleEndedIterator for Outputs {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.range.next_back().map(|offset| Wire(self.node, offset))
    }
}

impl FusedIterator for Outputs {}

/// Common trait for handles to a node.
/// Typically wrappers around [`NodeIndex`].
pub trait NodeHandle {
    /// Index of underlying node.
    fn node(&self) -> NodeIndex;
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Handle to a dataflow node which has a known number of value outputs
pub struct BuildHandle<T> {
    node_handle: T,
    num_value_outputs: usize,
}

impl<T: From<NodeIndex>> From<(NodeIndex, usize)> for BuildHandle<T> {
    fn from((node, num_value_outputs): (NodeIndex, usize)) -> Self {
        Self {
            node_handle: node.into(),
            num_value_outputs,
        }
    }
}

impl<T: NodeHandle> NodeHandle for BuildHandle<T> {
    fn node(&self) -> NodeIndex {
        self.node_handle.node()
    }
}

impl<T: NodeHandle> BuildHandle<T> {
    #[inline]
    /// Number of Value kind outputs from this node.
    fn num_value_outputs(&self) -> usize {
        self.num_value_outputs
    }

    #[inline]
    /// Return iterator over Value outputs.
    pub fn outputs(&self) -> Outputs {
        Outputs {
            node: self.node(),
            range: (0..self.num_value_outputs()),
        }
    }

    /// Attempt to cast outputs in to array of Wires.
    pub fn outputs_arr<const N: usize>(&self) -> [Wire; N] {
        self.outputs()
            .collect_vec()
            .try_into()
            .expect(&format!("Incorrect number of wires: {}", N)[..])
    }

    #[inline]
    /// Retrieve a [`Wire`] corresponding to the given offset.
    /// Does not check whether such a wire is valid for this node.
    pub fn out_wire(&self, offset: usize) -> Wire {
        Wire(self.node(), offset)
    }

    #[inline]
    /// Underlying node handle
    pub fn handle(&self) -> &T {
        &self.node_handle
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DerFrom, Debug)]
/// Handle to a [LeafOp](crate::ops::leaf::LeafOp).
pub struct OpID(NodeIndex);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DerFrom, Debug)]
/// Handle to a [DFG](crate::ops::dataflow::DataflowOp::DFG) node.
pub struct DfgID(NodeIndex);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DerFrom, Debug)]
/// Handle to a [CFG](crate::ops::controlflow::ControlFlowOp::CFG) node.
pub struct CfgID(NodeIndex);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DerFrom, Debug)]
/// Handle to a [def](crate::ops::module::ModuleOp::Def)
/// or [declare](crate::ops::module::ModuleOp::Declare) node.
pub struct FuncID(NodeIndex);

#[derive(DerFrom, Debug, Clone)]
/// Handle to a [NewType](crate::ops::module::ModuleOp::NewType) node.
pub struct NewTypeID {
    node: NodeIndex,
    name: SmolStr,
    core_type: SimpleType,
}

impl NewTypeID {
    /// Retrieve the NewType
    pub fn get_new_type(&self) -> SimpleType {
        self.core_type.clone().into_new_type(self.name.clone())
    }

    /// Retrieve the underlying core type
    pub fn get_core_type(&self) -> &SimpleType {
        &self.core_type
    }

    /// Retrieve the underlying core type
    pub fn get_name(&self) -> &SmolStr {
        &self.name
    }
}

#[derive(DerFrom, Debug)]
/// Handle to a [Const](crate::ops::module::ModuleOp::Const) node.
pub struct ConstID(NodeIndex, ClassicType);

impl ConstID {
    /// Return the type of the constant.
    pub fn const_type(&self) -> ClassicType {
        self.1.clone()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DerFrom, Debug)]
/// Handle to a [BasicBlock](crate::ops::controlflow::BasicBlockOp) node.
pub struct BasicBlockID(NodeIndex);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DerFrom, Debug)]
/// Handle to a [Case](crate::ops::controlflow::CaseOp) node.
pub struct CaseID(NodeIndex);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DerFrom, Debug)]
/// Handle to a [TailLoop](crate::ops::controlflow::ControlFlowOp::TailLoop) node.
pub struct TailLoopID(NodeIndex);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DerFrom, Debug)]
/// Handle to a [Conditional](crate::ops::controlflow::ControlFlowOp::Conditional) node.
pub struct ConditionalID(NodeIndex);

impl From<BuildHandle<DfgID>> for BuildHandle<FuncID> {
    #[inline]
    fn from(value: BuildHandle<DfgID>) -> Self {
        Self {
            node_handle: FuncID(value.node()),
            num_value_outputs: value.num_value_outputs,
        }
    }
}

impl From<BuildHandle<DfgID>> for BasicBlockID {
    #[inline]
    fn from(value: BuildHandle<DfgID>) -> Self {
        Self(value.node())
    }
}

impl From<BuildHandle<DfgID>> for BuildHandle<CaseID> {
    #[inline]
    fn from(value: BuildHandle<DfgID>) -> Self {
        Self {
            node_handle: CaseID(value.node()),
            num_value_outputs: value.num_value_outputs,
        }
    }
}

impl From<BuildHandle<DfgID>> for BuildHandle<TailLoopID> {
    #[inline]
    fn from(value: BuildHandle<DfgID>) -> Self {
        Self {
            node_handle: TailLoopID(value.node()),
            num_value_outputs: value.num_value_outputs,
        }
    }
}

impl NodeHandle for OpID {
    #[inline]
    fn node(&self) -> NodeIndex {
        self.0
    }
}

impl NodeHandle for ConditionalID {
    #[inline]
    fn node(&self) -> NodeIndex {
        self.0
    }
}

impl NodeHandle for DfgID {
    #[inline]
    fn node(&self) -> NodeIndex {
        self.0
    }
}

impl NodeHandle for TailLoopID {
    #[inline]
    fn node(&self) -> NodeIndex {
        self.0
    }
}

impl NodeHandle for CfgID {
    #[inline]
    fn node(&self) -> NodeIndex {
        self.0
    }
}

impl NodeHandle for FuncID {
    #[inline]
    fn node(&self) -> NodeIndex {
        self.0
    }
}

impl NodeHandle for NewTypeID {
    #[inline]
    fn node(&self) -> NodeIndex {
        self.node
    }
}

impl NodeHandle for ConstID {
    #[inline]
    fn node(&self) -> NodeIndex {
        self.0
    }
}

impl NodeHandle for BasicBlockID {
    #[inline]
    fn node(&self) -> NodeIndex {
        self.0
    }
}
