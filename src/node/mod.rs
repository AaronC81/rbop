//! Node tree data structures representing mathematical expressions in various formats.
//! 
//! The two kinds provided are [unstructured] and [structured] nodes. Unstructured nodes are suited
//! to elegant input and cursor navigation, but do not have enough structure to be evaluated.
//! Structured nodes follow a much more rigid format which prevents easy modification, but can be
//! evaluated easily. Unstructured nodes can be converted to structured nodes by
//! [upgrading](unstructured::Upgradable) them.
//! 
//! Both kinds of node support [rendering](crate::render).

pub mod unstructured;
pub mod structured;
pub mod simplified;
pub mod function;
mod parser;
mod common;