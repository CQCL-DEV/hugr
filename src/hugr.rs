//! The Hugr data structure, and its basic component handles.

mod hugrmut;

pub mod multiportgraph;
pub mod rewrite;
pub mod serialize;
pub mod typecheck;
pub mod validate;
pub mod view;

pub(crate) use self::hugrmut::HugrMut;
use self::multiportgraph::MultiPortGraph;
pub use self::validate::ValidationError;

use derive_more::From;
pub use rewrite::{Replace, ReplaceError, Rewrite, SimpleReplacement, SimpleReplacementError};

use portgraph::dot::{hier_graph_dot_string_with, DotEdgeStyle};
use portgraph::{Hierarchy, UnmanagedDenseMap};
use thiserror::Error;

pub use self::view::HugrView;
use crate::ops::{OpName, OpTrait, OpType};
use crate::types::EdgeKind;

use html_escape::encode_text_to_string;

/// The Hugr data structure.
#[derive(Clone, Debug, PartialEq)]
pub struct Hugr {
    /// The graph encoding the adjacency structure of the HUGR.
    graph: MultiPortGraph,

    /// The node hierarchy.
    hierarchy: Hierarchy,

    /// The single root node in the hierarchy.
    root: portgraph::NodeIndex,

    /// Operation types for each node.
    op_types: UnmanagedDenseMap<portgraph::NodeIndex, OpType>,
}

impl Default for Hugr {
    fn default() -> Self {
        Self::new(crate::ops::Module)
    }
}

impl AsRef<Hugr> for Hugr {
    fn as_ref(&self) -> &Hugr {
        self
    }
}

impl AsMut<Hugr> for Hugr {
    fn as_mut(&mut self) -> &mut Hugr {
        self
    }
}

/// A handle to a node in the HUGR.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, From)]
pub struct Node {
    index: portgraph::NodeIndex,
}

/// A handle to a port for a node in the HUGR.
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash, Default, Debug, From)]
pub struct Port {
    offset: portgraph::PortOffset,
}

/// The direction of a port.
pub type Direction = portgraph::Direction;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A DataFlow wire, defined by a Value-kind output port of a node
// Stores node and offset to output port
pub struct Wire(Node, usize);

/// Public API for HUGRs.
impl Hugr {
    /// Applies a rewrite to the graph.
    pub fn apply_rewrite<E>(&mut self, rw: impl Rewrite<E>) -> Result<(), E> {
        rw.apply(self)
    }

    /// Return dot string showing underlying graph and hierarchy side by side.
    pub fn dot_string(&self) -> String {
        let portgraph = self.graph.as_portgraph();
        hier_graph_dot_string_with(
            portgraph,
            &self.hierarchy,
            |n| {
                if !self.graph.contains_node(n) {
                    return "".into();
                }
                let name = self.op_types[n].name();
                format!("({ni}) {name}", ni = n.index())
            },
            |mut p| {
                let mut src = portgraph.port_node(p).unwrap();
                let src_is_copy = !self.graph.contains_node(src);
                let Some(tgt_port) = portgraph.port_link(p) else {
                        return ("".into(), DotEdgeStyle::None);
                    };
                let tgt = portgraph.port_node(tgt_port).unwrap();
                let tgt_is_copy = !self.graph.contains_node(tgt);
                if src_is_copy {
                    p = portgraph.input_links(src).next().unwrap().unwrap();
                    src = portgraph.port_node(p).unwrap();
                }

                let style =
                    if !tgt_is_copy && self.hierarchy.parent(src) != self.hierarchy.parent(tgt) {
                        DotEdgeStyle::Some("dashed".into())
                    } else if !src_is_copy
                        && self
                            .get_optype(src.into())
                            .port_kind(self.graph.port_offset(p).unwrap())
                            == Some(EdgeKind::StateOrder)
                    {
                        DotEdgeStyle::Some("dotted".into())
                    } else {
                        DotEdgeStyle::None
                    };

                let mut label = String::new();
                if !src_is_copy {
                    let optype = self.op_types.get(src);
                    let offset = portgraph.port_offset(p).unwrap();
                    let type_string = match optype.port_kind(offset) {
                        Some(EdgeKind::Const(ty)) => format!("{}", ty),
                        Some(EdgeKind::Value(ty)) => format!("{}", ty),
                        _ => String::new(),
                    };
                    encode_text_to_string(type_string, &mut label);
                }

                (label, style)
            },
        )
    }
}

/// Internal API for HUGRs, not intended for use by users.
impl Hugr {
    /// Create a new Hugr, with a single root node.
    pub(crate) fn new(root_op: impl Into<OpType>) -> Self {
        Self::with_capacity(root_op, 0, 0)
    }

    /// Create a new Hugr, with a single root node and preallocated capacity.
    pub(crate) fn with_capacity(root_op: impl Into<OpType>, nodes: usize, ports: usize) -> Self {
        let mut graph = MultiPortGraph::with_capacity(nodes, ports);
        let hierarchy = Hierarchy::new();
        let mut op_types = UnmanagedDenseMap::with_capacity(nodes);
        let root = graph.add_node(0, 0);
        op_types[root] = root_op.into();

        Self {
            graph,
            hierarchy,
            root,
            op_types,
        }
    }
}

impl Port {
    /// Creates a new port.
    #[inline]
    pub fn new(direction: Direction, port: usize) -> Self {
        Self {
            offset: portgraph::PortOffset::new(direction, port),
        }
    }

    /// Creates a new incoming port.
    #[inline]
    pub fn new_incoming(port: usize) -> Self {
        Self {
            offset: portgraph::PortOffset::new_incoming(port),
        }
    }

    /// Creates a new outgoing port.
    #[inline]
    pub fn new_outgoing(port: usize) -> Self {
        Self {
            offset: portgraph::PortOffset::new_outgoing(port),
        }
    }

    /// Returns the direction of the port.
    #[inline]
    pub fn direction(self) -> Direction {
        self.offset.direction()
    }

    /// Returns the offset of the port.
    #[inline(always)]
    pub fn index(self) -> usize {
        self.offset.index()
    }
}

impl Wire {
    /// Create a new wire from a node and a port.
    #[inline]
    pub fn new(node: Node, port: Port) -> Self {
        Self(node, port.index())
    }

    /// The node that this wire is connected to.
    #[inline]
    pub fn node(&self) -> Node {
        self.0
    }

    /// The output port that this wire is connected to.
    #[inline]
    pub fn source(&self) -> Port {
        Port::new_outgoing(self.1)
    }
}

/// Errors that can occur while manipulating a Hugr.
///
/// TODO: Better descriptions, not just re-exporting portgraph errors.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum HugrError {
    /// An error occurred while connecting nodes.
    #[error("An error occurred while connecting the nodes.")]
    ConnectionError(#[from] portgraph::LinkError),
    /// An error occurred while manipulating the hierarchy.
    #[error("An error occurred while manipulating the hierarchy.")]
    HierarchyError(#[from] portgraph::hierarchy::AttachError),
}
