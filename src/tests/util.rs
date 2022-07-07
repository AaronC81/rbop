macro_rules! uns_list {
    ($($x:expr),* $(,)?) => { crate::UnstructuredNodeList { items: alloc::vec![ $($x),* ] } };
}

macro_rules! token {
    (+)             => { crate::UnstructuredNode::Token(crate::Token::Add) };
    (-)             => { crate::UnstructuredNode::Token(crate::Token::Subtract) };
    (*)             => { crate::UnstructuredNode::Token(crate::Token::Multiply) };
    (/)             => { crate::UnstructuredNode::Token(crate::Token::Divide) };
    (.)             => { crate::UnstructuredNode::Token(crate::Token::Point) };
    (var $v:ident)  => { crate::UnstructuredNode::Token(crate::Token::Variable(stringify!($v).chars().nth(0).unwrap())) };
    ($x:literal)    => { crate::UnstructuredNode::Token(crate::Token::Digit($x)) };
}

macro_rules! tokens {
    ($($x:tt) *) => { crate::UnstructuredNodeList { items: alloc::vec![ $(token!($x)),* ] } };
}

macro_rules! uns_frac {
    ($t:expr, $b:expr $(,)?) => { crate::UnstructuredNode::Fraction($t, $b) };
}

macro_rules! render {
    ($n:expr, $p:expr, $v:expr $(,)?) => { {
        let mut renderer = crate::renderers::AsciiRenderer::default();
        <crate::renderers::AsciiRenderer as crate::render::Renderer>::draw_all(&mut renderer, &$n, $p, $v);
        renderer.lines
    } };

    ($n:expr, $p:expr $(,)?) => { render!($n, $p, None) };

    ($n:expr $(,)?) => { render!($n, None, None) };
}

macro_rules! rat {
    ($n:literal)             => { crate::Number::Rational($n, 1)  };
    ($n:literal, $d:literal) => { crate::Number::Rational($n, $d) };
}

macro_rules! dec {
    ($l:literal) => {
        crate::Number::Decimal(
            <rust_decimal::Decimal as core::str::FromStr>::from_str(stringify!($l)).unwrap(),
            crate::number::DecimalAccuracy::Exact,
        )
    };
}

macro_rules! dec_approx {
    ($l:literal) => {
        crate::Number::Decimal(
            <rust_decimal::Decimal as core::str::FromStr>::from_str(stringify!($l)).unwrap(),
            crate::number::DecimalAccuracy::Approximation,
        )
    }
}

macro_rules! reserialize {
    ($e:expr) => {
        <UnstructuredNodeRoot as crate::node::unstructured::Serializable>::deserialize(
            &mut <_ as crate::node::unstructured::Serializable>::serialize(&$e).into_iter()
        ).unwrap()
    };
}

macro_rules! reduce {
    ($n:expr) => {
        {
            let mut nodes = $n;
            assert!(matches!(nodes.reduce(), Ok(_)));
            nodes
        }
    };
}

macro_rules! simplify {
    ($t:expr) => {
        {
            let mut n =
                <_ as crate::node::simplified::Simplifiable>::simplify(
                    &<_ as crate::node::unstructured::Upgradable>::upgrade(&$t).unwrap()
                ).flatten();
            n.sort();
            n
        }
    };
}

/// ```text
///       56    
///    34+--
///       78   
/// 12+-----+12
///     90  
/// ```   
pub fn complex_unstructured_expression() -> crate::UnstructuredNodeRoot {
    crate::UnstructuredNodeRoot { root: uns_list!(
        token!(1),
        token!(2),
        token!(+),
        uns_frac!(
            uns_list!(
                token!(3),
                token!(4),
                token!(+),
                uns_frac!(
                    tokens!(5 6),
                    tokens!(7 8),
                )
            ),
            tokens!(9 0),
        ),
        token!(+),
        token!(1),
        token!(2),
    ) }
}
