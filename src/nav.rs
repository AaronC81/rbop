use alloc::{vec, vec::Vec};

use crate::{UnstructuredNodeList, render::{Layoutable, Renderer}, node::unstructured::Navigable};

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

impl core::ops::Index<usize> for NavPath {
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

pub enum MoveVerticalDirection {
    Up,
    Down,
}

/// Given two unstructured nodes which are vertically centre-aligned, and a direction in which
/// the cursor is moving, returns a vec of positions `v` such that moving the cursor from
/// from position `i` in that direction should put the cursor in position `v[i]` of the other
/// unstructured node. 
pub fn match_vertical_cursor_points(
    renderer: &mut impl Renderer,
    top: &UnstructuredNodeList,
    bottom: &UnstructuredNodeList,
    direction: MoveVerticalDirection
) -> Vec<usize> {
    let (from_node, to_node) = match direction {
        MoveVerticalDirection::Up => (bottom, top),
        MoveVerticalDirection::Down => (top, bottom),
    };

    // Render both nodes
    let mut from_layouts = from_node.items
        .iter()
        .map(|node| node.layout(renderer, None))
        .collect::<Vec<_>>();
    let mut to_layouts = to_node.items
        .iter()
        .map(|node| node.layout(renderer, None))
        .collect::<Vec<_>>();

    // Work out complete widths
    let from_total_width: u64 = from_layouts
        .iter()
        .map(|x| x.area.width)
        .sum();
    let to_total_width: u64 = to_layouts
        .iter()
        .map(|x| x.area.width)
        .sum();

    // Calculate some offsets to vertically centre them
    let (from_offset, to_offset) = if from_total_width < to_total_width {
        ((to_total_width - from_total_width) / 2, 0)
    } else if from_total_width > to_total_width {
        (0, (from_total_width - to_total_width) / 2)
    } else {
        (0, 0)
    };

    // Collect boundary points between the layout items
    let mut from_boundary_points = vec![from_offset];
    for layout in &from_layouts {
        from_boundary_points.push(
            from_boundary_points.last().unwrap() + layout.area.width
        )
    }
    let mut to_boundary_points = vec![to_offset];
    for layout in &to_layouts {
        to_boundary_points.push(
            to_boundary_points.last().unwrap() + layout.area.width
        )
    }
    
    // Go through each "from" item, and find the closest "to" item
    // O(n^2), whoops!
    let mut result = vec![];
    for from_point in from_boundary_points {
        let mut closest_to_idx_found = 0;

        for (i, to_point) in to_boundary_points.iter().enumerate() {
            let this_distance = (*to_point as i64 - from_point as i64).abs();
            let best_distance = (to_boundary_points[closest_to_idx_found] as i64 - from_point as i64).abs();
            if this_distance < best_distance {
                closest_to_idx_found = i;
            }
        }

        result.push(closest_to_idx_found);
    }

    result
}
