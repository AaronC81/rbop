#![feature(box_syntax)]
#![feature(box_patterns)]
#![feature(or_patterns)]

#![no_std]
extern crate core;
extern crate alloc;

pub mod node;
pub mod nav;
pub mod test;
pub mod render;
pub mod renderers;

pub trait Error : alloc::fmt::Display + alloc::fmt::Debug {}

pub use crate::node::{Node, Token};
