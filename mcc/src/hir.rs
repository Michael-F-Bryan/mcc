//! The *High-level Intermediate Representation*.

use heapsize::HeapSizeOf;
use heapsize_derive::HeapSizeOf;
use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;

#[derive(Debug, Default, Clone, PartialEq, HeapSizeOf)]
pub struct CompilationUnit {
    pub functions: HashMap<HirId, Function>,
    pub types: HashMap<HirId, Type>,
    pub namespace: HashMap<String, HirId>,
}

impl CompilationUnit {
    pub fn new() -> CompilationUnit {
        CompilationUnit::default()
    }

    pub fn lookup(&self, name: &str) -> Option<HirId> {
        self.namespace.get(name).cloned()
    }

    pub fn add_function(&mut self, func: Function) {
        let hir_id = func.node_id;
        let name = func.name.clone();

        debug_assert!(!self.namespace.contains_key(&name));
        debug_assert!(!self.functions.contains_key(&hir_id));

        self.namespace.insert(name, hir_id);
        self.functions.insert(hir_id, func);
    }
}

#[derive(Debug, Clone, PartialEq, HeapSizeOf)]
pub struct Function {
    pub node_id: HirId,
    pub name: String,
    pub ty: HirId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, HeapSizeOf)]
pub enum Type {
    Void,
    Pointer(HirId),
    Integral { signed: bool, bits: u8 },
    Float { bits: u8 },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, HeapSizeOf)]
pub struct HirId(u32);

impl HirId {
    pub(crate) fn new(n: u32) -> HirId {
        HirId(n)
    }

    pub fn placeholder() -> HirId {
        HirId::new(0)
    }

    pub fn is_valid(&self) -> bool {
        self.0 == 0
    }
}

pub trait HirNode: Any + Debug + HeapSizeOf {
    fn node_id(&self) -> HirId;
}

macro_rules! impl_hir_node {
    ($type:ty) => {
        impl $crate::hir::HirNode for $type {
            fn node_id(&self) -> HirId {
                self.node_id
            }
        }
    };

    ($type:ident; $( $variant:ident ),+) => {
        impl $crate::hir::HirNode for $type {
            fn node_id(&self) -> HirId {
                defer!($type, self; $($variant),+ => |item| item.node_id())
            }
        }
    };
}

impl_hir_node!(Function);
