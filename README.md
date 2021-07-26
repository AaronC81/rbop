# rbop

![Crates.io](https://img.shields.io/crates/v/rbop)

rbop (**R**ust **B**inary **Op**erations) is a framework for implementing intuitive mathematical
expression editors.

rbop is `no_std`, so you can use it pretty much anywhere. To create an editor for your particular
use-case, all you need to do is provide simple method implementations to draw core mathematical
glyphs. rbop will use these to calculate a two-dimensional layout and draw to your chosen canvas!

## Try it out

rbop comes with a simple ASCII renderer, which is used in an example named `ascii_calc`. If you
`cargo build --examples`, you'll be able to run this example and get a feel for how natural rbop's 
editor feels!

## Implementing a renderer

Refer to `AsciiRenderer` for a pretty good example of this. You'll need to implement the `Renderer`
trait, which will allow rbop to:

- Reset and prepare your graphics surface with `init`
- Determine the size which a particular glyph will be when drawn to your canvas, using `size`, to
  calculate a layout
- Draw glyphs to your canvas at a particular location with `draw`

Do not override the default implementations of any other methods in `Renderer`.

## Node trees

Though only one `Node` enum is used, rbop uses it to provide two different kinds of node trees.

An **unstructured** node tree is really easy to build through user inputs. Horizontal inputs are
left as token streams, so operator precedence does not need to be considered.

```
1+2*3             Fraction
-----             /      \
  5           1,+,2,*,3   5
```

A **structured** node tree contains no raw token streams, and the tree structure fully expresses the
correct operator precedence.

```
1+2*3             Fraction
-----             /      \
  5             Add       5
               /   \
              1    Mult
                  /    \
                 2      3
```

An unstructured node tree can be converted to a structured node tree by **upgrading** it. Some
functionality, such as evaluation, can only be performed on a structured node tree.
