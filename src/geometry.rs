use std::fmt::Debug;

use nalgebra::{
    ClosedAddAssign, ClosedSubAssign, Point, SVector, Scalar, SimdPartialOrd,
    Translation2, point, zero,
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

        pub trait SizeExt<T> {
            fn into_ratatui(self) -> ::ratatui::layout::Size;
            fn into_nalgebra(self) -> SVector<T, 2>;
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

#[derive(Clone, Copy)]
pub struct Rectangle<T: Scalar = u16> {
    // inclusive
    pub min: Point<T, 2>,
    // exclusive
    pub max: Point<T, 2>,
}

impl<T> Debug for Rectangle<T>
where
    T: Scalar + Debug + ClosedSubAssign,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rectangle")
            .field("min", &self.min)
            .field("max", &self.max)
            .field("extent", &self.extent())
            .finish()
    }
}

impl From<Rect> for Rectangle {
    fn from(value: Rect) -> Self {
        Self {
            min: point![value.x, value.y],
            max: point![value.x + value.width, value.y + value.height],
        }
    }
}

impl From<Rectangle> for Rect {
    fn from(value: Rectangle) -> Self {
        let extent = value.extent();
        Self {
            x: value.min.x,
            y: value.min.y,
            width: extent.x,
            height: extent.y,
        }
    }
}

impl<T: Scalar> Rectangle<T> {
    pub fn intersect(&self, rhs: &Self) -> Self
    where
        T: SimdPartialOrd,
    {
        Self {
            min: self.min.sup(&rhs.min),
            max: self.max.inf(&rhs.max),
        }
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
        R: Scalar,
        T: SubsetOf<R> + Copy,
    {
        Rectangle {
            min: self.min.cast::<R>(),
            max: self.max.cast::<R>(),
        }
    }

    pub fn try_cast<R>(&self) -> Option<Rectangle<R>>
    where
        R: Scalar + SubsetOf<T>,
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
