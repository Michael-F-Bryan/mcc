#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    pub start: usize,
    pub length: usize,
}

impl Span {
    pub const fn new(start: usize, length: usize) -> Self {
        Span { start, length }
    }

    pub const fn end(&self) -> usize {
        self.start + self.length
    }

    pub fn for_node(node: tree_sitter::Node<'_>) -> Self {
        node.range().into()
    }

    pub const fn to_range(self) -> std::ops::Range<usize> {
        self.start..self.end()
    }

    pub fn lookup(self, text: &str) -> &str {
        &text[self.to_range()]
    }
}

impl From<tree_sitter::Range> for Span {
    fn from(range: tree_sitter::Range) -> Self {
        Span::new(range.start_byte, range.end_byte - range.start_byte)
    }
}

impl From<std::ops::Range<usize>> for Span {
    fn from(value: std::ops::Range<usize>) -> Self {
        Span::new(value.start, value.len())
    }
}

impl From<Span> for std::ops::Range<usize> {
    fn from(value: Span) -> Self {
        value.to_range()
    }
}
