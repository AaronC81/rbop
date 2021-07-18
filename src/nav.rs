use crate::{Node};

/// Describes the movements which must be taken down a node tree to reach the position of the 
/// cursor.
pub struct NavPath {
    path: Vec<usize>,
}

impl NavPath {
    pub fn new(path: Vec<usize>) -> Self { Self { path } }

    pub fn to_navigator(&mut self) -> NavPathNavigator {
        NavPathNavigator {
            path: self,
            index: 0 
        }
    }
}

pub struct NavPathNavigator<'a> {
    path: &'a mut NavPath,
    index: usize,
}

impl<'a> NavPathNavigator<'a> {
    /// The next index in the path.
    pub fn next(&mut self) -> usize {
        self.path.path[self.index]
    }

    /// Returns true if there is only one index left in the path; in other words, the cursor must
    /// be in this node.
    pub fn here(&mut self) -> bool {
        self.index == self.path.path.len() - 1
    }

    /// Returns a copy of the path with the first item removed, making the path relative to one node
    /// deeper into the tree.
    pub fn step(&mut self) -> NavPathNavigator {
        NavPathNavigator { index: self.index + 1, path: self.path }
    }
}
