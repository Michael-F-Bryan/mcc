use heapsize_derive::HeapSizeOf;

/// A unique ID which corresponds to a particular AST node.
///
/// As a special case, `NodeId(0)` is an invalid node ID. This allows it to
/// be used as a placeholder.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, HeapSizeOf)]
pub struct NodeId(usize);

impl NodeId {
    fn new(n: usize) -> NodeId {
        NodeId(n)
    }

    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }

    pub fn placeholder() -> NodeId {
        NodeId::new(0)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct NodeIdGenerator {
    last_id: usize,
}

impl NodeIdGenerator {
    pub fn new() -> Self {
        NodeIdGenerator { last_id: 0 }
    }

    pub fn next_id(&mut self) -> NodeId {
        self.last_id += 1;
        NodeId::new(self.last_id)
    }
}
