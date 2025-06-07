use std::{fmt::Debug, ops::Range};

use nalgebra::{
    ClosedAddAssign, ClosedSubAssign, Point, SVector, Scalar, SimdPartialOrd, Translation2, point,
    vector, zero,
};
use num_traits::{SaturatingSub, Zero};
use ratatui::layout::Rect;
use simba::scalar::SubsetOf;

pub mod ext {
    use ::nalgebra::Scalar;
    use simba::scalar::SupersetOf;

    pub trait IntoRatatui<T> {
        type Output;
        fn into_ratatui_cast(self) -> Self::Output;
    }

    pub trait IntoTaffy<T> {
        type Output<U>;
        fn into_taffy_cast<U: SupersetOf<T>>(self) -> Self::Output<U>;
    }

    pub trait IntoNalgebra<T> {
        type Output<U: Scalar>;
        fn into_nalgebra_cast<U: SupersetOf<T> + Scalar>(self) -> Self::Output<U>;
    }

    pub trait IntoRatatuiExt<T>: IntoRatatui<T> {
        fn into_ratatui(self) -> Self::Output;
    }

    impl<T, U> IntoRatatuiExt<T> for U
    where
        U: IntoRatatui<T>,
    {
        fn into_ratatui(self) -> Self::Output {
            self.into_ratatui_cast()
        }
    }

    pub trait IntoTaffyExt<T>: IntoTaffy<T> {
        #[expect(unused)]
        fn into_taffy(self) -> Self::Output<T>;
    }

    impl<T, U> IntoTaffyExt<T> for U
    where
        U: IntoTaffy<T>,
        T: SupersetOf<T>,
    {
        fn into_taffy(self) -> Self::Output<T> {
            self.into_taffy_cast()
        }
    }

    pub trait IntoNalgebraExt<T>: IntoNalgebra<T>
    where
        T: Scalar,
    {
        fn into_nalgebra(self) -> Self::Output<T>;
    }

    impl<T, U> IntoNalgebraExt<T> for U
    where
        T: Scalar + SupersetOf<T>,
        U: IntoNalgebra<T>,
    {
        fn into_nalgebra(self) -> Self::Output<T> {
            self.into_nalgebra_cast()
        }
    }

    pub mod ratatui {
        use nalgebra::{Point, SVector, Scalar, convert, point, vector};
        use simba::scalar::SupersetOf;

        use super::{IntoNalgebra, IntoTaffy};

        impl IntoTaffy<u16> for ::ratatui::layout::Size {
            type Output<U> = ::taffy::Size<U>;

            fn into_taffy_cast<U: SupersetOf<u16>>(self) -> Self::Output<U> {
                Self::Output {
                    width: convert(self.width),
                    height: convert(self.height),
                }
            }
        }

        impl IntoNalgebra<u16> for ::ratatui::layout::Size {
            type Output<U: Scalar> = SVector<U, 2>;

            fn into_nalgebra_cast<U: SupersetOf<u16> + Scalar>(self) -> Self::Output<U> {
                vector![convert(self.width), convert(self.height)]
            }
        }

        impl IntoTaffy<u16> for ::ratatui::layout::Position {
            type Output<U> = ::taffy::Point<U>;

            fn into_taffy_cast<U: SupersetOf<u16>>(self) -> Self::Output<U> {
                Self::Output {
                    x: convert(self.x),
                    y: convert(self.y),
                }
            }
        }

        impl IntoNalgebra<u16> for ::ratatui::layout::Position {
            type Output<U: Scalar> = Point<U, 2>;

            fn into_nalgebra_cast<U: SupersetOf<u16> + Scalar>(self) -> Self::Output<U> {
                point![convert(self.x), convert(self.y)]
            }
        }

        impl IntoNalgebra<i32> for ::ratatui::layout::Offset {
            type Output<U: Scalar> = SVector<U, 2>;

            fn into_nalgebra_cast<U: SupersetOf<i32> + Scalar>(self) -> Self::Output<U> {
                vector![convert(self.x), convert(self.y)]
            }
        }
    }

    pub mod taffy {
        use nalgebra::{SVector, Scalar, convert, vector};
        use simba::scalar::{SubsetOf, SupersetOf};

        use super::{IntoNalgebra, IntoRatatui};

        impl<T> IntoNalgebra<T> for ::taffy::Size<T> {
            type Output<U: Scalar> = SVector<T, 2>;

            fn into_nalgebra_cast<U: SupersetOf<T> + Scalar>(self) -> Self::Output<U> {
                vector![self.width, self.height]
            }
        }

        impl<T> IntoRatatui<T> for ::taffy::Size<T>
        where
            T: SubsetOf<u16>,
        {
            type Output = ratatui::layout::Size;

            fn into_ratatui_cast(self) -> Self::Output {
                Self::Output {
                    width: convert(self.width),
                    height: convert(self.height),
                }
            }
        }
    }

    pub mod nalgebra {
        use nalgebra::{Point, SVector, Scalar, convert};
        use simba::scalar::{SubsetOf, SupersetOf};

        use super::{IntoRatatui, IntoTaffy};

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

        impl<T> IntoRatatui<T> for Point<T, 2>
        where
            T: SubsetOf<u16> + Scalar + Copy,
        {
            type Output = ::ratatui::layout::Position;

            fn into_ratatui_cast(self) -> Self::Output {
                Self::Output {
                    x: convert(self.x),
                    y: convert(self.y),
                }
            }
        }

        impl<T> IntoRatatui<T> for SVector<T, 2>
        where
            T: SubsetOf<u16> + Scalar + Copy,
        {
            type Output = ::ratatui::layout::Size;

            fn into_ratatui_cast(self) -> Self::Output {
                Self::Output {
                    width: convert(self.x),
                    height: convert(self.y),
                }
            }
        }

        impl<T> IntoTaffy<T> for Point<T, 2>
        where
            T: Scalar + Copy,
        {
            type Output<U> = ::taffy::Point<U>;

            fn into_taffy_cast<U: SupersetOf<T>>(self) -> Self::Output<U> {
                Self::Output::<U> {
                    x: convert(self.x),
                    y: convert(self.y),
                }
            }
        }

        impl<T> IntoTaffy<T> for SVector<T, 2>
        where
            T: Scalar + Copy,
        {
            type Output<U> = ::taffy::Size<U>;

            fn into_taffy_cast<U: SupersetOf<T>>(self) -> Self::Output<U> {
                Self::Output::<U> {
                    width: convert(self.x),
                    height: convert(self.y),
                }
            }
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Rectangle<T: Scalar + Zero = u16> {
    // inclusive
    min: Point<T, 2>,
    // exclusive
    max: Point<T, 2>,
}

impl<T> Debug for Rectangle<T>
where
    T: Scalar + Zero + Debug + ClosedSubAssign + Copy + SaturatingSub,
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
        T: ClosedSubAssign + SaturatingSub,
    {
        vector![
            self.max.x.saturating_sub(&self.min.x),
            self.max.y.saturating_sub(&self.min.y),
        ]
    }

    pub fn is_empty(&self) -> bool
    where
        T: PartialOrd + ClosedSubAssign + Zero + SaturatingSub,
    {
        self.extent().iter().any(|c| c <= &T::zero())
    }

    pub fn area(&self) -> T
    where
        T: ClosedSubAssign + ClosedAddAssign + Zero + SaturatingSub,
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
        // let min = Point {
        //     coords: self.min.coords.sup(&zero()),
        // };
        // let max = Point {
        //     coords: self.max.coords.sup(&min.coords),
        // };
        // Rectangle {
        //     min: min.try_cast::<u16>().unwrap(),
        //     max: max.try_cast::<u16>().unwrap(),
        // }
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
