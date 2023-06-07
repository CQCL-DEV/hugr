#![allow(missing_docs)]
//! Replace operations on Hugr graphs. This is a nonfunctional
//! dummy implementation just to demonstrate design principles.

use std::collections::HashMap;

use portgraph::substitute::OpenGraph;
use portgraph::{NodeIndex, PortIndex};
use thiserror::Error;

use super::Rewrite;
use crate::Hugr;

/// A subset of the nodes in a graph, and the ports that it is connected to.
#[derive(Debug, Clone, Default)]
pub struct BoundedSubgraph {
    /// Nodes in the subgraph.
    pub subgraph: portgraph::substitute::BoundedSubgraph,
}

impl BoundedSubgraph {
    /// Creates a new bounded subgraph.
    ///
    /// TODO: We should be able to automatically detect dangling ports by
    /// finding inputs and outputs in `hugr` that are connected to things
    /// outside. Can we do that efficiently?
    pub fn new(_hugr: &Hugr, _nodes: impl IntoIterator<Item = NodeIndex>) -> Self {
        todo!()
    }
}

/// A graph with explicit input and output ports.
#[derive(Clone, Default, Debug)]
pub struct OpenHugr {
    /// The graph.
    pub hugr: Hugr,
    /// Incoming dangling ports in the graph.
    pub dangling_inputs: Vec<PortIndex>,
    /// Outgoing dangling ports in the graph.
    pub dangling_outputs: Vec<PortIndex>,
}

impl OpenHugr {
    /// Creates a new open graph.
    ///
    /// TODO: We should be able to automatically detect dangling ports by
    /// finding inputs and outputs in `hugr` that are connected to things
    /// outside. Can we do that efficiently?
    pub fn new(_hugr: Hugr) -> Self {
        todo!()
    }

    /// Extracts the internal open graph, and returns the Hugr with additional components on the side.
    ///
    /// The returned Hugr will have no graph information.
    pub fn into_parts(self) -> (OpenGraph, Hugr) {
        let OpenHugr {
            hugr,
            dangling_inputs,
            dangling_outputs,
        } = self;
        let _ = (hugr, dangling_inputs, dangling_outputs);
        todo!("The internal graph of a hugr cannot be accessed directly. This needs updating.");
        //let graph = std::mem::take(&mut hugr.graph);
        //(
        //    OpenGraph {
        //        graph,
        //        dangling_inputs,
        //        dangling_outputs,
        //    },
        //    hugr,
        //)
    }
}

pub type ParentsMap = HashMap<NodeIndex, NodeIndex>;

/// A rewrite operation that replaces a subgraph with another graph.
/// Includes the new weights for the nodes in the replacement graph.
#[derive(Debug, Clone)]
pub struct Replace {
    /// The subgraph to be replaced.
    subgraph: BoundedSubgraph,
    /// The replacement graph.
    replacement: OpenHugr,
    /// A map from the nodes in the replacement graph to the target parents in the original graph.
    parents: ParentsMap,
}

impl Replace {
    /// Creates a new rewrite operation.
    pub fn new(
        subgraph: BoundedSubgraph,
        replacement: OpenHugr,
        parents: impl Into<ParentsMap>,
    ) -> Self {
        Self {
            subgraph,
            replacement,
            parents: parents.into(),
        }
    }

    /// Extracts the internal graph rewrite, and returns the replacement Hugr
    /// with additional components on the side.
    ///
    /// The returned Hugr will have no graph information.
    pub(crate) fn into_parts(self) -> (portgraph::substitute::Rewrite, Hugr, ParentsMap) {
        let (open_graph, replacement) = self.replacement.into_parts();
        (
            portgraph::substitute::Rewrite::new(self.subgraph.subgraph, open_graph),
            replacement,
            self.parents,
        )
    }

    pub fn verify_convexity(&self) -> Result<(), ReplaceError> {
        todo!()
    }

    pub fn verify_boundaries(&self) -> Result<(), ReplaceError> {
        todo!()
    }
}

impl Rewrite<ReplaceError> for Replace {
    const UNCHANGED_ON_FAILURE: bool = false;

    /// Checks that the rewrite is valid.
    ///
    /// This includes having a convex subgraph (TODO: include definition), and
    /// having matching numbers of ports on the boundaries.
    /// TODO not clear this implementation really provides much guarantee about [self.apply]
    /// but this class is not really working anyway.
    fn verify(&self, _h: &Hugr) -> Result<(), ReplaceError> {
        self.verify_convexity()?;
        self.verify_boundaries()?;
        Ok(())
    }

    /// Performs a Replace operation on the graph.
    fn apply(self, h: &mut Hugr) -> Result<(), ReplaceError> {
        // Get the open graph for the rewrites, and a HUGR with the additional components.
        let (rewrite, mut replacement, parents) = self.into_parts();

        // TODO: Use `parents` to update the hierarchy, and keep the internal hierarchy from `replacement`.
        let _ = parents;

        let node_inserted = |old, new| {
            std::mem::swap(&mut h.op_types[new], &mut replacement.op_types[old]);
            // TODO: metadata (Fn parameter ?)
        };
        // unchanged_on_failure is false, so no guarantees here
        rewrite.apply_with_callbacks(
            &mut h.graph,
            |_| {},
            |_| {},
            node_inserted,
            |_, _| {},
            |_, _| {},
        )?;

        // TODO: Check types
        Ok(())
    }
}

/// Error generated when a rewrite fails.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ReplaceError {
    /// The replacement failed because the boundary defined by the
    /// [`Replace`] could not be matched to the dangling ports of the
    /// [`OpenHugr`].
    #[error("The boundary defined by the rewrite could not be matched to the dangling ports of the OpenHugr")]
    BoundarySize(#[source] portgraph::substitute::RewriteError),
    /// There was an error connecting the ports of the [`OpenHugr`] to the
    /// boundary.
    #[error("An error occurred while connecting the ports of the OpenHugr to the boundary")]
    ConnectionError(#[source] portgraph::LinkError),
    /// The rewrite target is not convex
    ///
    /// TODO: include context.
    #[error("The rewrite target is not convex")]
    NotConvex(),
}

impl From<portgraph::substitute::RewriteError> for ReplaceError {
    fn from(e: portgraph::substitute::RewriteError) -> Self {
        match e {
            portgraph::substitute::RewriteError::BoundarySize => Self::BoundarySize(e),
            portgraph::substitute::RewriteError::Link(e) => Self::ConnectionError(e),
        }
    }
}