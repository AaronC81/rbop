use core::cmp::max;

use crate::{nav::NavPathNavigator, render::{Glyph, LayoutBlock, Layoutable, MergeBaseline, Renderer}};

pub fn layout_sqrt<T>(inner: &T, renderer: &mut impl Renderer, path: Option<&mut NavPathNavigator>) -> LayoutBlock
where T : Layoutable
{
    // Lay out the inner item first
    let mut path = if let Some(p) = path {
        if p.next() == 0 {
            Some(p.step())
        } else {
            None
        }
    } else {
        None
    };
    
    let inner_layout = inner.layout(renderer, (&mut path).as_mut());
    let inner_area = inner_layout.area(renderer);

    // Get glyph size for the sqrt symbol
    let sqrt_symbol_layout = LayoutBlock::from_glyph(renderer, Glyph::Sqrt {
        inner_area
    });

    // We assume that the inner layout goes in the very bottom right, so work out the
    // offset required based on the difference of the two areas
    let x_offset = sqrt_symbol_layout.area(renderer).width - inner_layout.area(renderer).width;
    let y_offset = sqrt_symbol_layout.area(renderer).height - inner_layout.area(renderer).height;

    // Merge the two
    sqrt_symbol_layout.merge_in_place(
        renderer, 
        &inner_layout.offset(x_offset, y_offset),
        MergeBaseline::OtherAsBaseline
    )
}

pub fn layout_fraction<T>(top: &T, bottom: &T, renderer: &mut impl Renderer, path: Option<&mut NavPathNavigator>) -> LayoutBlock
where T : Layoutable
{
    let (mut top_path, mut bottom_path) = {
        if let Some(p) = path {
            if p.next() == 0 {
                (Some(p.step()), None)
            } else if p.next() == 1 {
                (None, Some(p.step()))
            } else {
                panic!()
            }
        } else {
            (None, None)
        }
    };

    let top_layout = top.layout(
        renderer,
        (&mut top_path).as_mut()
    );
    let bottom_layout = bottom.layout(
        renderer,
        (&mut bottom_path).as_mut()
    );

    // The fraction line should be the widest of the two
    let line_width = max(
        top_layout.area(renderer).width,
        bottom_layout.area(renderer).width,
    );
    let line_layout = LayoutBlock::from_glyph(renderer, Glyph::Fraction {
        inner_width: line_width
    }).move_below_other(renderer, &top_layout);

    let bottom_layout = bottom_layout
        .move_below_other(renderer, &line_layout);

    top_layout
        .merge_along_vertical_centre(renderer, &line_layout, MergeBaseline::OtherAsBaseline)
        .merge_along_vertical_centre(renderer, &bottom_layout, MergeBaseline::SelfAsBaseline)
}

pub fn layout_parentheses<T>(inner: &T, renderer: &mut impl Renderer, path: Option<&mut NavPathNavigator>) -> LayoutBlock
where T : Layoutable
{
    // Lay out the inner item first
    let mut path = if let Some(p) = path {
        if p.next() == 0 {
            Some(p.step())
        } else {
            None
        }
    } else {
        None
    };
    
    let inner_layout = inner.layout(renderer, (&mut path).as_mut());
    let inner_area = inner_layout.area(renderer);

    // Get glyphs for parentheses
    let mut left_paren_layout = LayoutBlock::from_glyph(renderer, Glyph::LeftParenthesis {
        inner_height: inner_area.height,
    });
    let mut right_paren_layout = LayoutBlock::from_glyph(renderer, Glyph::RightParenthesis {
        inner_height: inner_area.height,
    });

    // Match the baselines for these glyphs with the inner baseline
    left_paren_layout.baseline = inner_layout.baseline;
    right_paren_layout.baseline = inner_layout.baseline;

    // Merge the three
    LayoutBlock::layout_horizontal(renderer, &[
        left_paren_layout,
        inner_layout,
        right_paren_layout,
    ])
}
