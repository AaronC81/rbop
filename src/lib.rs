#![feature(box_syntax)]
#![feature(box_patterns)]
#![feature(or_patterns)]
#![feature(test)]

#![no_std]
extern crate core;
extern crate alloc;
extern crate test;

pub mod error;
pub mod node;
pub mod nav;
pub mod tests;
pub mod render;
pub mod renderers;

pub use crate::node::{
    unstructured::{UnstructuredNode, Token, UnstructuredNodeList, UnstructuredItem},
    structured::StructuredNode,
};

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
