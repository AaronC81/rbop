//! The unstructured node tree, suited to user input.
//! 
//! Unstructured nodes have a very loose structure, using a list of [Token]s where possible, only
//! introducing nodes with a more rigid layout where verticality is required (e.g. fractions).
//! 
//! These can not be evaluated directly; they need to be [upgraded](Upgradable) to a tree of
//! [structured](crate::node::structured) nodes first.

mod node;
pub use node::*;

mod layout;
pub use layout::*;

mod navigation;
pub use navigation::*;

mod upgrade;
pub use upgrade::*;

mod serialize;
pub use serialize::*;
