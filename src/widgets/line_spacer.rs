use std::io::Read;

use ratatui::{
    buffer::Buffer,
    layout::{Direction, Position, Rect},
    widgets::WidgetRef,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum LineType {
    #[default]
    None,
    Standard,
    Bold,
}

impl LineType {
    const fn index(&self) -> usize {
        match self {
            Self::None => 0,
            Self::Standard => 1,
            Self::Bold => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Symbol {
    top: LineType,
    right: LineType,
    bottom: LineType,
    left: LineType,
}

impl Symbol {
    pub const fn new(top: LineType, right: LineType, bottom: LineType, left: LineType) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub fn union(&self, other: &Self) -> Self {
        Self {
            top: std::cmp::max(self.top, other.top),
            right: std::cmp::max(self.right, other.right),
            bottom: std::cmp::max(self.bottom, other.bottom),
            left: std::cmp::max(self.left, other.left),
        }
    }

    const fn index(&self) -> usize {
        ((self.top.index() * 3 + self.right.index()) * 3 + self.bottom.index()) * 3
            + self.left.index()
    }

    fn draw(&self, buffer: &mut Buffer, position: Position) {
        if let Some(cell) = buffer.cell_mut(position) {
            let existing_symbol = {
                let mut chars = cell.symbol().chars();
                chars.next().and_then(move |character| {
                    if chars.next().is_some() {
                        None
                    } else {
                        Symbol::try_from(character).ok()
                    }
                })
            };
            let symbol = existing_symbol
                .map(|existing_symbol| existing_symbol.union(self))
                .unwrap_or_else(|| *self);
            let character = char::from(symbol);

            cell.set_char(character);
        }
    }
}

const CHAR_TO_SYMBOL_FIRST: char = '─';
const CHAR_TO_SYMBOL_LAST: char = '╿';
const CHAR_TO_SYMBOL_LEN: usize = CHAR_TO_SYMBOL_LAST as usize - CHAR_TO_SYMBOL_FIRST as usize + 1;
const CHAR_TO_SYMBOL: [Option<Symbol>; CHAR_TO_SYMBOL_LEN] = char_to_symbol_table();
const SYMBOL_TO_CHAR_LEN: usize = 3 * 3 * 3 * 3;
const SYMBOL_TO_CHAR: [char; SYMBOL_TO_CHAR_LEN] = symbol_to_char_table();

const fn symbol_to_char_table() -> [char; SYMBOL_TO_CHAR_LEN] {
    let mut table = ['\0'; SYMBOL_TO_CHAR_LEN];
    let mut character_index = CHAR_TO_SYMBOL_FIRST as u32;
    let last = CHAR_TO_SYMBOL_LAST as u32;

    loop {
        let character = char::from_u32(character_index).unwrap();
        if let Some(symbol) = char_to_symbol_slow(character) {
            table[symbol.index()] = character;
        }

        if character_index == last {
            break;
        }

        character_index += 1;
    }

    table
}

const fn char_to_symbol_table() -> [Option<Symbol>; CHAR_TO_SYMBOL_LEN] {
    let mut table = [None; CHAR_TO_SYMBOL_LEN];
    let mut index = 0;
    let mut character = CHAR_TO_SYMBOL_FIRST as u32;
    let last = CHAR_TO_SYMBOL_LAST as u32;

    loop {
        table[index] = char_to_symbol_slow(char::from_u32(character).unwrap());

        if character == last {
            break;
        }

        index += 1;
        character += 1;
    }

    table
}

const fn char_to_symbol_slow(character: char) -> Option<Symbol> {
    use LineType::*;

    match character {
        '─' => Some(Symbol::new(None, Standard, None, Standard)),
        '━' => Some(Symbol::new(None, Bold, None, Bold)),
        '│' => Some(Symbol::new(Standard, None, Standard, None)),
        '┃' => Some(Symbol::new(Bold, None, Bold, None)),
        '┌' => Some(Symbol::new(None, Standard, Standard, None)),
        '┍' => Some(Symbol::new(None, Bold, Standard, None)),
        '┎' => Some(Symbol::new(None, Standard, Bold, None)),
        '┏' => Some(Symbol::new(None, Bold, Bold, None)),
        '┐' => Some(Symbol::new(None, None, Standard, Standard)),
        '┑' => Some(Symbol::new(None, None, Standard, Bold)),
        '┒' => Some(Symbol::new(None, None, Bold, Standard)),
        '┓' => Some(Symbol::new(None, None, Bold, Bold)),
        '└' => Some(Symbol::new(Standard, Standard, None, None)),
        '┕' => Some(Symbol::new(Standard, Bold, None, None)),
        '┖' => Some(Symbol::new(Bold, Standard, None, None)),
        '┗' => Some(Symbol::new(Bold, Bold, None, None)),
        '┘' => Some(Symbol::new(Standard, None, None, Standard)),
        '┙' => Some(Symbol::new(Standard, None, None, Bold)),
        '┚' => Some(Symbol::new(Bold, None, None, Standard)),
        '┛' => Some(Symbol::new(Bold, None, None, Bold)),
        '├' => Some(Symbol::new(Standard, Standard, Standard, None)),
        '┝' => Some(Symbol::new(Standard, Bold, Standard, None)),
        '┞' => Some(Symbol::new(Bold, Standard, Standard, None)),
        '┟' => Some(Symbol::new(Standard, Standard, Bold, None)),
        '┠' => Some(Symbol::new(Bold, Standard, Bold, None)),
        '┡' => Some(Symbol::new(Bold, Bold, Standard, None)),
        '┢' => Some(Symbol::new(Standard, Bold, Bold, None)),
        '┣' => Some(Symbol::new(Bold, Bold, Bold, None)),
        '┤' => Some(Symbol::new(Standard, None, Standard, Standard)),
        '┥' => Some(Symbol::new(Standard, None, Standard, Bold)),
        '┦' => Some(Symbol::new(Bold, None, Standard, Standard)),
        '┧' => Some(Symbol::new(Standard, None, Bold, Standard)),
        '┨' => Some(Symbol::new(Bold, None, Bold, Standard)),
        '┩' => Some(Symbol::new(Bold, None, Standard, Bold)),
        '┪' => Some(Symbol::new(Standard, None, Bold, Bold)),
        '┫' => Some(Symbol::new(Bold, None, Bold, Bold)),
        '┬' => Some(Symbol::new(None, Standard, Standard, Standard)),
        '┭' => Some(Symbol::new(None, Standard, Standard, Bold)),
        '┮' => Some(Symbol::new(None, Bold, Standard, Standard)),
        '┯' => Some(Symbol::new(None, Bold, Standard, Bold)),
        '┰' => Some(Symbol::new(None, Standard, Bold, Standard)),
        '┱' => Some(Symbol::new(None, Standard, Bold, Bold)),
        '┲' => Some(Symbol::new(None, Bold, Bold, Standard)),
        '┳' => Some(Symbol::new(None, Bold, Bold, Bold)),
        '┴' => Some(Symbol::new(Standard, Standard, None, Standard)),
        '┵' => Some(Symbol::new(Standard, Standard, None, Bold)),
        '┶' => Some(Symbol::new(Standard, Bold, None, Standard)),
        '┷' => Some(Symbol::new(Standard, Bold, None, Bold)),
        '┸' => Some(Symbol::new(Bold, Standard, None, Standard)),
        '┹' => Some(Symbol::new(Bold, Standard, None, Bold)),
        '┺' => Some(Symbol::new(Bold, Bold, None, Standard)),
        '┻' => Some(Symbol::new(Bold, Bold, None, Bold)),
        '┼' => Some(Symbol::new(Standard, Standard, Standard, Standard)),
        '┽' => Some(Symbol::new(Standard, Standard, Standard, Bold)),
        '┾' => Some(Symbol::new(Standard, Bold, Standard, Standard)),
        '┿' => Some(Symbol::new(Standard, Bold, Standard, Bold)),
        '╀' => Some(Symbol::new(Bold, Standard, Standard, Standard)),
        '╁' => Some(Symbol::new(Standard, Standard, Bold, Standard)),
        '╂' => Some(Symbol::new(Bold, Standard, Bold, Standard)),
        '╃' => Some(Symbol::new(Bold, Standard, Standard, Bold)),
        '╄' => Some(Symbol::new(Bold, Bold, Standard, Standard)),
        '╅' => Some(Symbol::new(Standard, Standard, Bold, Bold)),
        '╆' => Some(Symbol::new(Standard, Bold, Bold, Standard)),
        '╇' => Some(Symbol::new(Bold, Bold, Standard, Bold)),
        '╈' => Some(Symbol::new(Standard, Bold, Bold, Bold)),
        '╉' => Some(Symbol::new(Bold, Standard, Bold, Bold)),
        '╊' => Some(Symbol::new(Bold, Bold, Bold, Standard)),
        '╋' => Some(Symbol::new(Bold, Bold, Bold, Bold)),
        '╴' => Some(Symbol::new(None, None, None, Standard)),
        '╵' => Some(Symbol::new(Standard, None, None, None)),
        '╶' => Some(Symbol::new(None, Standard, None, None)),
        '╷' => Some(Symbol::new(None, None, Standard, None)),
        '╸' => Some(Symbol::new(None, None, None, Bold)),
        '╹' => Some(Symbol::new(Bold, None, None, None)),
        '╺' => Some(Symbol::new(None, Bold, None, None)),
        '╻' => Some(Symbol::new(None, None, Bold, None)),
        '╼' => Some(Symbol::new(None, Bold, None, Standard)),
        '╽' => Some(Symbol::new(Standard, None, Bold, None)),
        '╾' => Some(Symbol::new(None, Standard, None, Bold)),
        '╿' => Some(Symbol::new(Bold, None, Standard, None)),
        _ => Option::None,
    }
}

impl TryFrom<char> for Symbol {
    type Error = ();

    fn try_from(character: char) -> Result<Self, Self::Error> {
        if character < CHAR_TO_SYMBOL_FIRST || character > CHAR_TO_SYMBOL_LAST {
            return Err(());
        }

        let index = character as usize - CHAR_TO_SYMBOL_FIRST as usize;

        CHAR_TO_SYMBOL[index].ok_or(())
    }
}

impl From<Symbol> for char {
    fn from(value: Symbol) -> Self {
        SYMBOL_TO_CHAR[value.index()]
    }
}

pub struct LineSpacer {
    pub direction: Direction,
    pub line_type: LineType,
}

impl WidgetRef for LineSpacer {
    fn render_ref(&self, area: Rect, buffer: &mut Buffer) {
        match self.direction {
            Direction::Horizontal => {
                if area.height == 0 || area.width <= 1 {
                    return;
                }

                Symbol {
                    right: self.line_type,
                    ..Default::default()
                }
                .draw(buffer, Position::new(area.x, area.y));
                Symbol {
                    left: self.line_type,
                    ..Default::default()
                }
                .draw(buffer, Position::new(area.x + area.width - 1, area.y));

                for x in ((area.x + 1)..).take(area.width as usize - 2) {
                    Symbol {
                        left: self.line_type,
                        right: self.line_type,
                        ..Default::default()
                    }
                    .draw(buffer, Position::new(x, area.y));
                }
            }
            Direction::Vertical => {
                if area.width == 0 || area.height <= 1 {
                    return;
                }

                Symbol {
                    bottom: self.line_type,
                    ..Default::default()
                }
                .draw(buffer, Position::new(area.x, area.y));
                Symbol {
                    top: self.line_type,
                    ..Default::default()
                }
                .draw(buffer, Position::new(area.x, area.y + area.height - 1));

                for y in ((area.y + 1)..).take(area.height as usize - 2) {
                    Symbol {
                        top: self.line_type,
                        bottom: self.line_type,
                        ..Default::default()
                    }
                    .draw(buffer, Position::new(area.x, y));
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct RectSpacer {
    pub line_type: LineType,
}

impl WidgetRef for RectSpacer {
    fn render_ref(&self, area: Rect, buffer: &mut Buffer) {
        let horizontal = LineSpacer {
            direction: Direction::Horizontal,
            line_type: self.line_type,
        };
        let vertical = LineSpacer {
            direction: Direction::Vertical,
            line_type: self.line_type,
        };

        horizontal.render_ref(area, buffer);
        vertical.render_ref(area, buffer);
        horizontal.render_ref(
            Rect {
                x: area.x,
                y: area.y + area.height - 1,
                width: area.width,
                height: 1,
            },
            buffer,
        );
        vertical.render_ref(
            Rect {
                x: area.x + area.width - 1,
                y: area.y,
                width: 1,
                height: area.height,
            },
            buffer,
        );
    }
}

#[derive(Clone, Debug)]
pub struct LineSpacerOld {
    pub direction: Direction,
    pub begin: &'static str,
    pub inner: &'static str,
    pub end: &'static str,
    pub merged: &'static str,
}

impl WidgetRef for LineSpacerOld {
    fn render_ref(&self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        //debug_assert!(
        //    (self.direction == Direction::Horizontal || area.width == 1)
        //        && (self.direction == Direction::Vertical || area.height == 1),
        //    "Invalid render area: direction = {direction:?}, area = {area:?}",
        //    direction = self.direction
        //);

        let start_position = area.as_position();

        if area.width == 0 || area.height == 0 {
            return;
        }

        if area.width <= 1 && area.height <= 1 {
            buf[start_position].set_symbol(self.merged);
            return;
        }

        buf[start_position].set_symbol(self.begin);

        match self.direction {
            Direction::Horizontal => {
                let end_position: Position =
                    (start_position.x + area.width - 1, start_position.y).into();
                buf[end_position].set_symbol(self.end);
                for x in (start_position.x + 1)..end_position.x {
                    let position = (x, start_position.y);
                    buf[position].set_symbol(self.inner);
                }
            }
            Direction::Vertical => {
                let end_position: Position =
                    (start_position.x, start_position.y + area.height - 1).into();
                buf[end_position].set_symbol(self.end);
                for y in (start_position.y + 1)..end_position.y {
                    let position = (start_position.x, y);
                    buf[position].set_symbol(self.inner);
                }
            }
        }
    }
}
