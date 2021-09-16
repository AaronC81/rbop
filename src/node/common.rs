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
    let inner_area = inner_layout.area;

    // Get glyph size for the sqrt symbol
    let sqrt_symbol_layout = LayoutBlock::from_glyph(renderer, Glyph::Sqrt {
        inner_area
    });

    // We assume that the inner layout goes in the very bottom right, so work out the
    // offset required based on the difference of the two areas
    let x_offset = sqrt_symbol_layout.area.width - inner_layout.area.width;
    let y_offset = sqrt_symbol_layout.area.height - inner_layout.area.height;

    // Merge the two
    sqrt_symbol_layout.merge_in_place(
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
        top_layout.area.width,
        bottom_layout.area.width,
    );
    let line_layout = LayoutBlock::from_glyph(renderer, Glyph::Fraction {
        inner_width: line_width
    }).move_below_other(&top_layout);

    let bottom_layout = bottom_layout
        .move_below_other(&line_layout);

    top_layout
        .merge_along_vertical_centre(&line_layout, MergeBaseline::OtherAsBaseline)
        .merge_along_vertical_centre(&bottom_layout, MergeBaseline::SelfAsBaseline)
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
    let inner_area = inner_layout.area;

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
    LayoutBlock::layout_horizontal(&[
        left_paren_layout,
        inner_layout,
        right_paren_layout,
    ])
}

pub fn layout_power<T>(base: &T, exp: &T, renderer: &mut impl Renderer, path: Option<&mut NavPathNavigator>) -> LayoutBlock
where T : Layoutable
{
    // Calculate paths for base and exponent
    let (mut base_path, mut exp_path) = {
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

    // Lay out base and exponent
    let base_layout = base.layout(
        renderer,
        (&mut base_path).as_mut()
    );
    let mut exp_layout = exp.layout(
        renderer,
        (&mut exp_path).as_mut()
    );

    // We're going to merge with `merge_in_place`, and want this:
    //
    //   12
    //   --
    //   34
    //  9
    //
    // Since both layouts start at (0, 0), what we've currently got is this:
    //
    //  ?2   <-- ? = 1 and 9 on top of each other
    //  --
    //  34
    //
    // The easiest solution would be to offset the exponent right and up, but it isn't possible to
    // offset upwards because offsets are required to be Dimensions, and Dimensions are unsigned.
    //
    // Instead:
    //   - Move the exponent right by the width of the base
    //   - Move the base down by the height of the exponent
    let base_layout = base_layout.offset(
        0,
        exp_layout.area.height
    );
    let exp_layout = exp_layout.offset(
        base_layout.area.width,
        0,
    );
    base_layout.merge_in_place(&exp_layout, MergeBaseline::SelfAsBaseline)
}
