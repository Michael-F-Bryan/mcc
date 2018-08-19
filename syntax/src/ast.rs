pub struct File {

}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(usize);

impl NodeId {
    fn new(n: usize) -> NodeId {
        NodeId(n)
    }

    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct NodeIdGenerator {
    last_id: usize,
}

impl NodeIdGenerator {
    pub fn new() -> Self {
        NodeIdGenerator {
            last_id: 0,
        }
    }

    pub fn next_id(&mut self) -> NodeId {
        self.last_id += 1;
        NodeId::new(self.last_id)
    }
}
