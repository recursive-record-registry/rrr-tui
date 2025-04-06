use ratatui::{
    layout::{Rect, Size},
    widgets::Padding,
};

pub enum LineAlignment {
    Start,
    End,
}

pub struct PlaneAlignment {
    pub x: LineAlignment,
    pub y: LineAlignment,
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
                LineAlignment::End => self.x + self.width.saturating_sub(rect_size.width),
            },
            y: match alignment.y {
                LineAlignment::Start => self.y,
                LineAlignment::End => self.y + self.height.saturating_sub(rect_size.height),
            },
            width: std::cmp::min(self.width, rect_size.width),
            height: std::cmp::min(self.height, rect_size.height),
        }
    }
}
