//! The Hugr data structure, and its basic component handles.

mod hugrmut;

#[cfg(feature = "patternmatching")]
pub mod circuit_hugr;
pub mod multiportgraph;
pub mod serialize;
pub mod typecheck;
pub mod validate;
pub mod view;

use std::collections::HashMap;

pub(crate) use self::hugrmut::HugrMut;
use self::multiportgraph::MultiPortGraph;
pub use self::validate::ValidationError;

use derive_more::From;
use itertools::Itertools;
use portgraph::dot::{hier_graph_dot_string_with, DotEdgeStyle};
use portgraph::{Hierarchy, NodeIndex, PortGraph, UnmanagedDenseMap};
use thiserror::Error;

pub use self::view::HugrView;
use crate::ops::tag::OpTag;
use crate::ops::{LeafOp, OpName, OpTrait, OpType};
use crate::replacement::{SimpleReplacement, SimpleReplacementError};
use crate::rewrite::{Rewrite, RewriteError};
use crate::types::EdgeKind;
#[cfg(feature = "patternmatching")]
pub use circuit_hugr::CircuitHugr;

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
    pub(crate) index: portgraph::NodeIndex,
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
    /// Apply a simple replacement operation to the HUGR.
    pub fn apply_simple_replacement(
        &mut self,
        r: SimpleReplacement,
    ) -> Result<(), SimpleReplacementError> {
        // 1. Check the parent node exists and is a DFG node.
        if self.get_optype(r.parent).tag() != OpTag::Dfg {
            return Err(SimpleReplacementError::InvalidParentNode());
        }
        // 2. Check that all the to-be-removed nodes are children of it and are leaves.
        for node in &r.removal {
            if self.hierarchy.parent(node.index) != Some(r.parent.index)
                || self.hierarchy.has_children(node.index)
            {
                return Err(SimpleReplacementError::InvalidRemovedNode());
            }
        }
        // 3. Do the replacement.
        // 3.1. Add copies of all replacement nodes and edges to self. Exclude Input/Output nodes.
        // Create map from old NodeIndex (in r.replacement) to new NodeIndex (in self).
        let mut index_map: HashMap<NodeIndex, NodeIndex> = HashMap::new();
        let replacement_nodes = r
            .replacement
            .children(r.replacement.root())
            .collect::<Vec<Node>>();
        // slice of nodes omitting Input and Output:
        let replacement_inner_nodes = &replacement_nodes[2..];
        for &node in replacement_inner_nodes {
            // Check there are no const inputs.
            if !r
                .replacement
                .get_optype(node)
                .signature()
                .const_input
                .is_empty()
            {
                return Err(SimpleReplacementError::InvalidReplacementNode());
            }
        }
        let self_output_node_index = self.children(r.parent).nth(1).unwrap();
        let replacement_output_node = *replacement_nodes.get(1).unwrap();
        for &node in replacement_inner_nodes {
            // Add the nodes.
            let op: &OpType = r.replacement.get_optype(node);
            let new_node_index = self
                .add_op_after(self_output_node_index, op.clone())
                .unwrap();
            index_map.insert(node.index, new_node_index.index);
        }
        // Add edges between all newly added nodes matching those in replacement.
        // TODO This will probably change when implicit copies are implemented.
        for &node in replacement_inner_nodes {
            let new_node_index = index_map.get(&node.index).unwrap();
            for node_successor in r.replacement.output_neighbours(node).unique() {
                if r.replacement.get_optype(node_successor).tag() != OpTag::Output {
                    let new_node_successor_index = index_map.get(&node_successor.index).unwrap();
                    for connection in r
                        .replacement
                        .graph
                        .get_connections(node.index, node_successor.index)
                    {
                        let src_offset = r
                            .replacement
                            .graph
                            .port_offset(connection.0)
                            .unwrap()
                            .index();
                        let tgt_offset = r
                            .replacement
                            .graph
                            .port_offset(connection.1)
                            .unwrap()
                            .index();
                        self.graph
                            .link_nodes(
                                *new_node_index,
                                src_offset,
                                *new_node_successor_index,
                                tgt_offset,
                            )
                            .ok();
                    }
                }
            }
        }
        // 3.2. For each p = r.nu_inp[q] such that q is not an Output port, add an edge from the
        // predecessor of p to (the new copy of) q.
        for ((rep_inp_node, rep_inp_port), (rem_inp_node, rem_inp_port)) in &r.nu_inp {
            if r.replacement.get_optype(*rep_inp_node).tag() != OpTag::Output {
                let new_inp_node_index = index_map.get(&rep_inp_node.index).unwrap();
                // add edge from predecessor of (s_inp_node, s_inp_port) to (new_inp_node, n_inp_port)
                let rem_inp_port_index = self
                    .graph
                    .port_index(rem_inp_node.index, rem_inp_port.offset)
                    .unwrap();
                let rem_inp_predecessor_subport = self.graph.port_link(rem_inp_port_index).unwrap();
                let rem_inp_predecessor_port_index = rem_inp_predecessor_subport.port();
                let new_inp_port_index = self
                    .graph
                    .port_index(*new_inp_node_index, rep_inp_port.offset)
                    .unwrap();
                self.graph.unlink_subport(rem_inp_predecessor_subport);
                self.graph
                    .link_ports(rem_inp_predecessor_port_index, new_inp_port_index)
                    .ok();
            }
        }
        // 3.3. For each q = r.nu_out[p] such that the predecessor of q is not an Input port, add an
        // edge from (the new copy of) the predecessor of q to p.
        for ((rem_out_node, rem_out_port), rep_out_port) in &r.nu_out {
            let rem_out_port_index = self
                .graph
                .port_index(rem_out_node.index, rem_out_port.offset)
                .unwrap();
            let rep_out_port_index = r
                .replacement
                .graph
                .port_index(replacement_output_node.index, rep_out_port.offset)
                .unwrap();
            let rep_out_predecessor_port_index =
                r.replacement.graph.port_link(rep_out_port_index).unwrap();
            let rep_out_predecessor_node_index = r
                .replacement
                .graph
                .port_node(rep_out_predecessor_port_index)
                .unwrap();
            if r.replacement
                .get_optype(rep_out_predecessor_node_index.into())
                .tag()
                != OpTag::Input
            {
                let rep_out_predecessor_port_offset = r
                    .replacement
                    .graph
                    .port_offset(rep_out_predecessor_port_index)
                    .unwrap();
                let new_out_node_index = index_map.get(&rep_out_predecessor_node_index).unwrap();
                let new_out_port_index = self
                    .graph
                    .port_index(*new_out_node_index, rep_out_predecessor_port_offset)
                    .unwrap();
                self.graph.unlink_port(rem_out_port_index);
                self.graph
                    .link_ports(new_out_port_index, rem_out_port_index)
                    .ok();
            }
        }
        // 3.4. For each q = r.nu_out[p1], p0 = r.nu_inp[q], add an edge from the predecessor of p0
        // to p1.
        for ((rem_out_node, rem_out_port), &rep_out_port) in &r.nu_out {
            let rem_inp_nodeport = r.nu_inp.get(&(replacement_output_node, rep_out_port));
            if let Some((rem_inp_node, rem_inp_port)) = rem_inp_nodeport {
                // add edge from predecessor of (rem_inp_node, rem_inp_port) to (rem_out_node, rem_out_port):
                let rem_inp_port_index = self
                    .graph
                    .port_index(rem_inp_node.index, rem_inp_port.offset)
                    .unwrap();
                let rem_inp_predecessor_port_index =
                    self.graph.port_link(rem_inp_port_index).unwrap().port();
                let rem_out_port_index = self
                    .graph
                    .port_index(rem_out_node.index, rem_out_port.offset)
                    .unwrap();
                self.graph.unlink_port(rem_inp_port_index);
                self.graph.unlink_port(rem_out_port_index);
                self.graph
                    .link_ports(rem_inp_predecessor_port_index, rem_out_port_index)
                    .ok();
            }
        }
        // 3.5. Remove all nodes in r.removal and edges between them.
        for node in &r.removal {
            self.graph.remove_node(node.index);
            self.hierarchy.remove(node.index);
        }
        Ok(())
    }

    /// Applies a rewrite to the graph.
    pub fn apply_rewrite(self, _rewrite: Rewrite) -> Result<(), RewriteError> {
        unimplemented!()
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

    /// HUGR as a simple weighted portgraph.
    ///
    /// Very naive, assumes the HUGR has no hierarchy.
    // TODO: do not rebuild leaf_ops every time
    pub fn as_weighted_graph(&self) -> (&PortGraph, UnmanagedDenseMap<NodeIndex, Option<LeafOp>>) {
        let mut leaf_ops = UnmanagedDenseMap::new();

        for n in self.graph.nodes_iter() {
            let op = &self.op_types[n];
            if let OpType::LeafOp(leaf_op) = op {
                leaf_ops[n] = leaf_op.clone().into();
            }
        }

        (self.graph.as_portgraph(), leaf_ops)
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
