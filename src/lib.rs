#![feature(box_syntax)]
#![feature(box_patterns)]
#![feature(or_patterns)]
#![feature(test)]

#![no_std]
extern crate core;
extern crate alloc;

#[cfg(test)]
extern crate test;

pub mod error;
pub mod node;
pub mod nav;
pub mod render;
pub mod renderers;
pub mod numeric;

#[cfg(test)]
pub mod tests;

pub use crate::node::{
    unstructured::{UnstructuredNode, Token, UnstructuredNodeList, UnstructuredItem},
    structured::StructuredNode,
};

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
