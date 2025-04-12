use ratatui::{
    layout::{Rect, Size},
    widgets::Padding,
};

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

    pub fn vertical(vertical_alignment: LineAlignment) -> Self {
        Self {
            x: Default::default(),
            y: vertical_alignment,
        }
    }
}

pub trait RectExt {
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
