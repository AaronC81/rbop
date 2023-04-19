#![feature(box_patterns)]
#![feature(test)]
#![feature(core_intrinsics)]
#![feature(if_let_guard)]
#![feature(assert_matches)]
#![feature(let_chains)]

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
pub mod decimal_ext;
pub mod number;
pub mod serialize;
pub mod evaluate;

#[cfg(test)]
mod tests;

pub use crate::{
    number::Number,
    node::{
        unstructured::{UnstructuredNode, Token, UnstructuredNodeList, UnstructuredItem, UnstructuredNodeRoot},
        structured::StructuredNode,
    }
};

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
