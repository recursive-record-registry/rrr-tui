use nalgebra::{ClosedAddAssign, ClosedDivAssign, ClosedSubAssign, Scalar, vector};
use num_traits::{NumCast, SaturatingSub, Zero};
use ratatui::{
    layout::{Rect, Size},
    widgets::Padding,
};

use crate::geometry::Rectangle;

#[derive(Default)]
pub enum LineAlignment {
    #[default]
    Start,
    Center,
    End,
}

pub struct PlaneAlignment {
    pub x: LineAlignment,
    pub y: LineAlignment,
}

impl PlaneAlignment {
    pub fn horizontal(horizontal_alignment: LineAlignment) -> Self {
        Self {
            x: horizontal_alignment,
            y: Default::default(),
        }
    }

    #[expect(unused)]
    pub fn vertical(vertical_alignment: LineAlignment) -> Self {
        Self {
            x: Default::default(),
            y: vertical_alignment,
        }
    }
}

pub trait RectExt {
    #[expect(unused)]
    fn without_padding(self, padding: Padding) -> Self;
    fn align(self, rect_size: Size, alignment: PlaneAlignment) -> Self;
}

impl RectExt for Rect {
    fn without_padding(self, padding: Padding) -> Self {
        Self {
            x: self.x + padding.left,
            y: self.y + padding.top,
            width: self.width.saturating_sub(padding.left + padding.right),
            height: self.height.saturating_sub(padding.top + padding.bottom),
        }
    }

    fn align(self, rect_size: Size, alignment: PlaneAlignment) -> Self {
        Self {
            x: match alignment.x {
                LineAlignment::Start => self.x,
                LineAlignment::Center => self.x + self.width.saturating_sub(rect_size.width) / 2,
                LineAlignment::End => self.x + self.width.saturating_sub(rect_size.width),
            },
            y: match alignment.y {
                LineAlignment::Start => self.y,
                LineAlignment::Center => self.y + self.height.saturating_sub(rect_size.height) / 2,
                LineAlignment::End => self.y + self.height.saturating_sub(rect_size.height),
            },
            width: std::cmp::min(self.width, rect_size.width),
            height: std::cmp::min(self.height, rect_size.height),
        }
    }
}

impl<T> RectExt for Rectangle<T>
where
    T: Scalar
        + Zero
        + NumCast
        + ClosedAddAssign
        + Copy
        + ClosedSubAssign
        + SaturatingSub
        + ClosedDivAssign
        + Ord,
{
    fn without_padding(self, padding: Padding) -> Self {
        Self::from_extent(
            self.min()
                + vector![
                    T::from(padding.left).unwrap(),
                    T::from(padding.right).unwrap()
                ],
            [
                self.extent()
                    .x
                    .saturating_sub(&T::from(padding.left + padding.right).unwrap()),
                self.extent()
                    .y
                    .saturating_sub(&T::from(padding.top + padding.bottom).unwrap()),
            ],
        )
    }

    fn align(self, rect_size: Size, alignment: PlaneAlignment) -> Self {
        Self::from_extent(
            [
                match alignment.x {
                    LineAlignment::Start => self.min().x,
                    LineAlignment::Center => {
                        self.min().x
                            + self
                                .extent()
                                .x
                                .saturating_sub(&T::from(rect_size.width).unwrap())
                                / T::from(2).unwrap()
                    }
                    LineAlignment::End => {
                        self.min().x
                            + self
                                .extent()
                                .x
                                .saturating_sub(&T::from(rect_size.width).unwrap())
                    }
                },
                match alignment.y {
                    LineAlignment::Start => self.min().y,
                    LineAlignment::Center => {
                        self.min().y
                            + self
                                .extent()
                                .y
                                .saturating_sub(&T::from(rect_size.height).unwrap())
                                / T::from(2).unwrap()
                    }
                    LineAlignment::End => {
                        self.min().y
                            + self
                                .extent()
                                .y
                                .saturating_sub(&T::from(rect_size.height).unwrap())
                    }
                },
            ],
            [
                std::cmp::min(self.extent().x, T::from(rect_size.width).unwrap()),
                std::cmp::min(self.extent().y, T::from(rect_size.height).unwrap()),
            ],
        )
    }
}
