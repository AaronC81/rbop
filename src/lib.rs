#![feature(box_syntax)]
#![feature(box_patterns)]
#![feature(or_patterns)]

#![no_std]
extern crate core;
extern crate alloc;

pub mod error;
pub mod node;
pub mod nav;
pub mod test;
pub mod render;
pub mod renderers;

pub use crate::node::{
    unstructured::{UnstructuredNode, Token, UnstructuredNodeList, UnstructuredItem},
    structured::StructuredNode,
};
