use test::{Bencher, black_box};

use crate::{render::{LayoutComputationProperties, Layoutable}, renderers::AsciiRenderer};

use super::util::complex_unstructured_expression;

#[bench]
fn bench_unstructured_layout(b: &mut Bencher) {
    let tree = complex_unstructured_expression();
    let mut ascii_renderer = AsciiRenderer::default();

    b.iter(|| {
        black_box(tree.layout(&mut ascii_renderer, None, LayoutComputationProperties::default()));
    });
}
