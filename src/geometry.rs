use std::{fmt::Debug, ops::Range};

use nalgebra::{
    ClosedAddAssign, ClosedSubAssign, Point, SVector, Scalar, SimdPartialOrd, Translation2, point,
    vector, zero,
};
use num_traits::Zero;
use ratatui::layout::{Offset, Position, Rect};
use simba::scalar::SubsetOf;

use crate::component::ComponentId;

pub mod ext {
    pub mod ratatui {
        use nalgebra::{SVector, vector};

        pub trait SizeExt {
            fn into_taffy<T: From<u16>>(self) -> taffy::Size<T>;
            fn into_nalgebra(self) -> SVector<u16, 2>;
        }

        impl SizeExt for ratatui::layout::Size {
            fn into_taffy<T: From<u16>>(self) -> ::taffy::Size<T> {
                ::taffy::Size {
                    width: self.width.into(),
                    height: self.height.into(),
                }
            }

            fn into_nalgebra(self) -> SVector<u16, 2> {
                vector![self.width, self.height]
            }
        }
    }

    pub mod taffy {
        use nalgebra::{SVector, vector};

        pub trait SizeExtNalgebra<T> {
            fn into_nalgebra(self) -> SVector<T, 2>;
        }

        pub trait SizeExt<T> {
            fn into_ratatui(self) -> ::ratatui::layout::Size;
        }

        impl<T> SizeExt<T> for ::taffy::Size<T>
        where
            T: Into<u16>,
        {
            fn into_ratatui(self) -> ::ratatui::layout::Size {
                ::ratatui::layout::Size {
                    width: self.width.into(),
                    height: self.height.into(),
                }
            }
        }

        impl<T> SizeExtNalgebra<T> for ::taffy::Size<T> {
            fn into_nalgebra(self) -> SVector<T, 2> {
                vector![self.width, self.height]
            }
        }

        pub trait RoundSizeExt<T> {
            fn rounded_into_ratatui(self) -> ::ratatui::layout::Size;
        }

        impl<T> RoundSizeExt<T> for ::taffy::Size<T>
        where
            T: num_traits::NumCast,
        {
            fn rounded_into_ratatui(self) -> ::ratatui::layout::Size {
                ::ratatui::layout::Size {
                    width: num_traits::cast(self.width).unwrap(),
                    height: num_traits::cast(self.height).unwrap(),
                }
            }
        }
    }

    pub mod nalgebra {
        use nalgebra::{Point, SVector, Scalar};
        use ratatui::layout::{Position, Size};
        use simba::scalar::SubsetOf;

        pub trait PointExt<T> {
            type TryCastResult<R>
            where
                R: Scalar;

            fn try_cast<R>(&self) -> Option<Self::TryCastResult<R>>
            where
                R: Scalar + SubsetOf<T>,
                T: Copy;
        }

        impl<T: Scalar, const D: usize> PointExt<T> for Point<T, D> {
            type TryCastResult<R>
                = Point<R, D>
            where
                R: Scalar;

            fn try_cast<R>(&self) -> Option<Self::TryCastResult<R>>
            where
                R: Scalar + SubsetOf<T>,
                T: Copy,
            {
                Some(Point {
                    coords: self.coords.try_cast::<R>()?,
                })
            }
        }

        pub trait PointExtRatatui<T> {
            fn into_ratatui(self) -> T;
        }

        impl PointExtRatatui<Position> for Point<u16, 2> {
            fn into_ratatui(self) -> Position {
                Position {
                    x: self.x,
                    y: self.y,
                }
            }
        }

        impl PointExtRatatui<Size> for SVector<u16, 2> {
            fn into_ratatui(self) -> Size {
                Size {
                    width: self.x,
                    height: self.y,
                }
            }
        }
    }

    pub use taffy::*;
}

impl From<ComponentId> for taffy::NodeId {
    fn from(value: ComponentId) -> Self {
        taffy::NodeId::new(value.0)
    }
}

impl From<taffy::NodeId> for ComponentId {
    fn from(value: taffy::NodeId) -> Self {
        ComponentId(value.into())
    }
}

pub trait PositionExt {
    fn as_offset(self) -> Offset;
    fn into_nalgebra(self) -> Point<u16, 2>;
}

impl PositionExt for Position {
    fn as_offset(self) -> Offset {
        Offset {
            x: self.x as i32,
            y: self.y as i32,
        }
    }

    fn into_nalgebra(self) -> Point<u16, 2> {
        point![self.x, self.y]
    }
}

#[derive(Clone, Copy, Default)]
pub struct Rectangle<T: Scalar + Zero = u16> {
    // inclusive
    min: Point<T, 2>,
    // exclusive
    max: Point<T, 2>,
}

impl<T> Debug for Rectangle<T>
where
    T: Scalar + Zero + Debug + ClosedSubAssign + Copy,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rectangle")
            .field("min", &self.min())
            .field("max", &self.max())
            .field("extent", &self.extent())
            .finish()
    }
}

impl From<Rect> for Rectangle {
    fn from(value: Rect) -> Self {
        Self::from_extent(point![value.x, value.y], vector![value.width, value.height])
    }
}

impl From<Rectangle> for Rect {
    fn from(value: Rectangle) -> Self {
        let min = value.min();
        let extent = value.extent();
        Self {
            x: min.x,
            y: min.y,
            width: extent.x,
            height: extent.y,
        }
    }
}

impl<T: Scalar + Zero, P: Into<Point<T, 2>>> From<Range<P>> for Rectangle<T> {
    fn from(value: Range<P>) -> Self {
        Self::from_minmax(value.start.into(), value.end.into())
    }
}

impl<T: Scalar + Zero> Rectangle<T> {
    pub fn from_minmax(min: impl Into<Point<T, 2>>, max: impl Into<Point<T, 2>>) -> Self {
        Self {
            min: min.into(),
            max: max.into(),
        }
    }

    pub fn from_extent(min: impl Into<Point<T, 2>>, extent: impl Into<SVector<T, 2>>) -> Self
    where
        T: ClosedAddAssign,
    {
        let min = min.into();
        Self {
            max: &min + extent.into(),
            min,
        }
    }

    pub fn intersect(&self, rhs: &Self) -> Self
    where
        T: SimdPartialOrd,
    {
        Self {
            min: self.min.sup(&rhs.min),
            max: self.max.inf(&rhs.max),
        }
    }

    pub fn set_min(&mut self, min: impl Into<Point<T, 2>>) {
        self.min = min.into();
    }

    pub fn set_max(&mut self, max: impl Into<Point<T, 2>>) {
        self.max = max.into();
    }

    pub fn set_extent(&mut self, extent: impl Into<SVector<T, 2>>)
    where
        T: ClosedAddAssign,
    {
        self.max = &self.min + extent.into();
    }

    pub fn with_extent(mut self, extent: impl Into<SVector<T, 2>>) -> Self
    where
        T: ClosedAddAssign,
    {
        self.set_extent(extent);
        self
    }

    pub fn set_width(&mut self, width: T)
    where
        T: ClosedAddAssign + Copy,
    {
        self.max.x = self.min.x + width;
    }

    pub fn with_width(mut self, width: T) -> Self
    where
        T: ClosedAddAssign + Copy,
    {
        self.set_width(width);
        self
    }

    pub fn set_height(&mut self, height: T)
    where
        T: ClosedAddAssign + Copy,
    {
        self.max.y = self.min.y + height;
    }

    pub fn with_height(mut self, height: T) -> Self
    where
        T: ClosedAddAssign + Copy,
    {
        self.set_height(height);
        self
    }

    pub fn min(&self) -> Point<T, 2>
    where
        T: Copy,
    {
        self.min
    }

    pub fn max(&self) -> Point<T, 2>
    where
        T: Copy,
    {
        self.max
    }

    pub fn extent(&self) -> SVector<T, 2>
    where
        T: ClosedSubAssign,
    {
        &self.max - &self.min
    }

    pub fn is_empty(&self) -> bool
    where
        T: PartialOrd + ClosedSubAssign + Zero,
    {
        self.extent().iter().any(|c| c <= &T::zero())
    }

    pub fn area(&self) -> T
    where
        T: ClosedSubAssign + ClosedAddAssign + Zero,
    {
        self.extent().sum()
    }

    pub fn cast<R>(&self) -> Rectangle<R>
    where
        R: Scalar + Zero,
        T: SubsetOf<R> + Copy,
    {
        Rectangle {
            min: self.min.cast::<R>(),
            max: self.max.cast::<R>(),
        }
    }

    pub fn try_cast<R>(&self) -> Option<Rectangle<R>>
    where
        R: Scalar + Zero + SubsetOf<T>,
        T: Copy,
    {
        Some(Rectangle {
            min: Point {
                coords: self.min.coords.try_cast::<R>()?,
            },
            max: Point {
                coords: self.max.coords.try_cast::<R>()?,
            },
        })
    }

    pub fn translated(&self, vec: SVector<T, 2>) -> Self
    where
        T: ClosedAddAssign,
    {
        Self {
            min: Translation2::from(vec.clone()).transform_point(&self.min),
            max: Translation2::from(vec).transform_point(&self.max),
        }
    }

    pub fn contains(&self, point: Point<T, 2>) -> bool
    where
        T: PartialOrd,
    {
        point.x >= self.min.x
            && point.x < self.max.x
            && point.y >= self.min.y
            && point.y < self.max.y
    }
}

impl Rectangle<i16> {
    pub fn clip(&self) -> Rectangle<u16> {
        Rectangle {
            min: Point {
                coords: self.min.coords.sup(&zero()).try_cast::<u16>().unwrap(),
            },
            max: Point {
                coords: self.max.coords.sup(&zero()).try_cast::<u16>().unwrap(),
            },
        }
    }
}
