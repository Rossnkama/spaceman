//! Space left empty around tiles: the `[gaps]` config table.
//!
//! The layout tree keeps partitioning one rectangle into touching cells. Gaps
//! are applied around it: the display is inset by the outer gaps and grown by
//! half an inner gap on every side, the tree partitions that area, and each
//! cell is then shrunk by half an inner gap per side to give its tile. Tiles at
//! the edge land exactly on the outer gaps and neighbors sit one inner gap
//! apart, at every nesting depth.

use serde::{Deserialize, Serialize};

use crate::Rect;

/// Gaps in points. All default to zero.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Gaps {
    /// Between neighboring tiles.
    #[serde(default)]
    pub inner: InnerGaps,
    /// Between tiles and the edges of the display's usable area.
    #[serde(default)]
    pub outer: OuterGaps,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InnerGaps {
    /// Between tiles that sit side by side.
    #[serde(default)]
    pub horizontal: u16,
    /// Between tiles that sit one above the other.
    #[serde(default)]
    pub vertical: u16,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OuterGaps {
    #[serde(default)]
    pub left: u16,
    #[serde(default)]
    pub bottom: u16,
    #[serde(default)]
    pub top: u16,
    #[serde(default)]
    pub right: u16,
}

impl Gaps {
    #[must_use]
    pub fn is_zero(&self) -> bool {
        *self == Self::default()
    }

    /// `output` without the outer gaps: the frame of a lone or Fullscreen Layout
    /// tile. Outer gaps that would leave no room on an axis are ignored on it.
    #[must_use]
    pub fn inset(self, output: Rect) -> Rect {
        let (x, width) = shrink(
            output.x,
            output.width,
            self.outer.left.into(),
            self.outer.right.into(),
        );
        let (y, height) = shrink(
            output.y,
            output.height,
            self.outer.top.into(),
            self.outer.bottom.into(),
        );
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    /// The rectangle the layout tree partitions into cells for `output`.
    pub(crate) fn layout_area(self, output: Rect) -> Rect {
        self.cell(self.inset(output))
    }

    /// The tile inside a layout cell. An inner gap as large as the cell is
    /// ignored on that axis, so a tile never collapses.
    pub(crate) fn tile(self, cell: Rect) -> Rect {
        let (left, right) = halves(self.inner.horizontal);
        let (top, bottom) = halves(self.inner.vertical);
        let (x, width) = shrink(cell.x, cell.width, left, right);
        let (y, height) = shrink(cell.y, cell.height, top, bottom);
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    /// The layout cell around a tile's frame: the inverse of [`Gaps::tile`].
    pub(crate) fn cell(self, tile: Rect) -> Rect {
        let (left, right) = halves(self.inner.horizontal);
        let (top, bottom) = halves(self.inner.vertical);
        Rect {
            x: tile.x - left,
            y: tile.y - top,
            width: tile.width + left + right,
            height: tile.height + top + bottom,
        }
    }

    /// A window's minimum size as the minimum size of its cell.
    pub(crate) fn cell_minimum(self, (width, height): (i32, i32)) -> (i32, i32) {
        (
            width.saturating_add(self.inner.horizontal.into()),
            height.saturating_add(self.inner.vertical.into()),
        )
    }
}

/// The two sides of one inner gap. They add up to it, so an odd gap stays exact.
fn halves(gap: u16) -> (i32, i32) {
    let leading = i32::from(gap / 2);
    (leading, i32::from(gap) - leading)
}

fn shrink(start: i32, extent: i32, leading: i32, trailing: i32) -> (i32, i32) {
    if extent - leading - trailing > 0 {
        (start + leading, extent - leading - trailing)
    } else {
        (start, extent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OUTPUT: Rect = Rect {
        x: 100,
        y: 50,
        width: 1000,
        height: 800,
    };

    fn gaps(inner: (u16, u16), outer: u16) -> Gaps {
        Gaps {
            inner: InnerGaps {
                horizontal: inner.0,
                vertical: inner.1,
            },
            outer: OuterGaps {
                left: outer,
                bottom: outer,
                top: outer,
                right: outer,
            },
        }
    }

    #[test]
    fn zero_gaps_change_nothing() {
        let none = Gaps::default();
        assert!(none.is_zero());
        assert_eq!(none.inset(OUTPUT), OUTPUT);
        assert_eq!(none.layout_area(OUTPUT), OUTPUT);
        assert_eq!(none.tile(OUTPUT), OUTPUT);
        assert_eq!(none.cell_minimum((300, 200)), (300, 200));
    }

    #[test]
    fn each_outer_gap_moves_its_own_edge() {
        let gaps = Gaps {
            outer: OuterGaps {
                left: 10,
                bottom: 20,
                top: 30,
                right: 40,
            },
            ..Gaps::default()
        };
        assert_eq!(
            gaps.inset(OUTPUT),
            Rect {
                x: 110,
                y: 80,
                width: 950,
                height: 750,
            }
        );
    }

    #[test]
    fn a_lone_cell_fills_the_inset_output_and_odd_gaps_stay_exact() {
        for gaps in [gaps((8, 8), 12), gaps((7, 5), 0), gaps((1, 0), 3)] {
            let area = gaps.layout_area(OUTPUT);
            assert_eq!(gaps.tile(area), gaps.inset(OUTPUT));
            assert_eq!(gaps.cell(gaps.tile(area)), area);

            // Two cells meeting at any x leave exactly one horizontal gap.
            let left = gaps.tile(Rect { width: 400, ..area });
            let right = gaps.tile(Rect {
                x: area.x + 400,
                width: area.width - 400,
                ..area
            });
            assert_eq!(
                right.x - (left.x + left.width),
                i32::from(gaps.inner.horizontal)
            );
            assert_eq!(left.x, gaps.inset(OUTPUT).x);
            assert_eq!(
                right.x + right.width,
                gaps.inset(OUTPUT).x + gaps.inset(OUTPUT).width
            );
        }
    }

    #[test]
    fn gaps_that_leave_no_room_are_ignored_on_that_axis() {
        let huge = gaps((0, 0), 500);
        assert_eq!(huge.inset(OUTPUT), OUTPUT);
        let cell = Rect {
            width: 40,
            height: 300,
            ..Rect::default()
        };
        let tile = gaps((40, 10), 0).tile(cell);
        assert_eq!((tile.x, tile.width), (0, 40));
        assert_eq!((tile.y, tile.height), (5, 290));
    }
}
