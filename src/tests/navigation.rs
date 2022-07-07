use alloc::vec;

use crate::{nav::NavPath, UnstructuredNodeList, node::unstructured::Navigable, UnstructuredNode, tests::util::complex_unstructured_expression, renderers::AsciiRenderer, Token};

#[test]
fn test_navigation() {
    let mut unstructured = uns_list!(
        token!(1),
        token!(2),
        token!(*),
        token!(3),
        token!(4),
        token!(+),
        uns_frac!(
            tokens!(5 6),
            tokens!(7 8),
        )
    );

    // Path 1: beginning
    let mut path = NavPath::new(vec![0]);
    let result = {
        let (node, i) = unstructured.navigate(&mut path.to_navigator());
        let node_ptr: *mut UnstructuredNodeList = node;
        (node_ptr, i)
    };
    assert_eq!(
        result,
        (&mut unstructured as *mut UnstructuredNodeList, 0)
    );

    // Path 2: middle
    let mut path = NavPath::new(vec![3]);
    let result = {
        let (node, i) = unstructured.navigate(&mut path.to_navigator());
        let node_ptr: *mut UnstructuredNodeList = node;
        (node_ptr, i)
    };
    assert_eq!(
        result,
        (&mut unstructured as *mut UnstructuredNodeList, 3)
    );

    // Path 3: nested
    let mut path = NavPath::new(vec![6, 1, 1]);
    let result = {
        let (node, i) = unstructured.navigate(&mut path.to_navigator());
        (node.clone(), i)
    };
    assert_eq!(
        result,
        (tokens!(7 8), 1)
    );
}

#[test]
fn test_movement() {
    let mut node = complex_unstructured_expression();
    let mut nav_path = NavPath::new(vec![0]);
    let mut renderer = AsciiRenderer::default();

    // Go all the way to the right
    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![1]));

    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3]));

    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 0]));

    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3]));

    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3, 0, 0]));

    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3, 0, 2]));

    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 4]));

    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![4]));

    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![5]));

    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![7]));

    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![7]));

    // Now go back to the left
    node.move_left(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![6]));

    node.move_left(&mut nav_path, &mut renderer, None);
    node.move_left(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![4]));

    node.move_left(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 4]));

    node.move_left(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3, 0, 2]));

    node.move_left(&mut nav_path, &mut renderer, None);
    node.move_left(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3, 0, 0]));

    node.move_left(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3]));

    node.move_left(&mut nav_path, &mut renderer, None);
    node.move_left(&mut nav_path, &mut renderer, None);
    node.move_left(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 0]));

    node.move_left(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3]));

    node.move_left(&mut nav_path, &mut renderer, None);
    node.move_left(&mut nav_path, &mut renderer, None);
    node.move_left(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![0]));

    // Move into a fraction to test vertical movement
    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 0]));
    
    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3, 0, 0]));
    
    // Now in a fraction, test vertical movement
    node.move_up(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3, 0, 0]));

    node.move_down(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3, 1, 0]));

    node.move_up(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3, 0, 0]));

    node.move_down(&mut nav_path, &mut renderer, None);
    node.move_down(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 1, 2]));

    node.move_up(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3]));
}

#[test]
fn test_modification() {
    let mut node = complex_unstructured_expression();
    let mut nav_path = NavPath::new(vec![0]);
    let mut renderer = AsciiRenderer::default();

    node.insert(&mut nav_path, &mut renderer, None, UnstructuredNode::Token(Token::Digit(1)));
    assert_eq!(nav_path, NavPath::new(vec![1]));

    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![4, 0, 0]));

    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![4, 0, 2]));

    node.insert(&mut nav_path, &mut renderer, None, UnstructuredNode::Token(Token::Add));
    node.insert(&mut nav_path, &mut renderer, None, UnstructuredNode::Fraction(
        UnstructuredNodeList::new(),
        UnstructuredNodeList::new(),
    ));
    node.insert(&mut nav_path, &mut renderer, None, UnstructuredNode::Token(Token::Digit(9)));
    assert_eq!(nav_path, NavPath::new(vec![4, 0, 3, 0, 1]));

    assert_eq!(
        render!(node, Some(&mut nav_path.to_navigator())),
        vec![
            "       9| 56   ",
            "    34+--+--   ",
            "        X 78   ",
            "112+--------+12",
            "       90      "
        ],
    );

    // Now try deleting some bits
    node.delete(&mut nav_path, &mut renderer, None);
    node.delete(&mut nav_path, &mut renderer, None);
    node.delete(&mut nav_path, &mut renderer, None);

    assert_eq!(
        render!(node, Some(&mut nav_path.to_navigator())),
        vec![
            "        56   ",
            "    34|+--   ",
            "        78   ",
            "112+------+12",
            "      90     "
        ],
    );
}
