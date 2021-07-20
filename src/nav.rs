use crate::{Node};

/// Describes the movements which must be taken down a node tree to reach the position of the 
/// cursor.
#[derive(PartialEq, Eq, Debug)]
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

    /// Returns true if the entire path only has one item.
    pub fn root(&mut self) -> bool {
        self.path.len() == 1
    }    

    /// Removes n entries from this path. May invalidate navigators, be careful!
    pub fn pop(&mut self, n: usize) {
        for _ in 0..n {
            self.path.pop();
        }
    }

    /// Adds index to this path.
    pub fn push(&mut self, index: usize) {
        self.path.push(index);
    }

    /// Adds n to the final entry of this path. This will not navigate deep into node structures,
    /// you should use `Node`'s `move_x` methods for this.
    pub fn offset(&mut self, n: isize) {
        *self.path.last_mut().unwrap() = (*self.path.last().unwrap() as isize + n) as usize;
    }
}

impl std::ops::Index<usize> for NavPath {
    type Output = usize;

    fn index(&self, index: usize) -> &Self::Output {
        &self.path[index]
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

    /// Helper method for Renderer. Returns a copy created by `step`, if `next` returns the given
    /// value.
    pub fn step_if_next(&mut self, required_next: usize) -> Option<NavPathNavigator> {
        if self.next() == required_next {
            Some(self.step())
        } else {
            None
        }
    }
}
