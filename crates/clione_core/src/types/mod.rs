/// A signed terminal/grid coordinate.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Point {
    /// Horizontal coordinate.
    pub x: i32,
    /// Vertical coordinate.
    pub y: i32,
}

impl Point {
    /// Creates a point.
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// A non-negative terminal/grid size.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Size {
    /// Width in cells.
    pub width: u16,
    /// Height in cells.
    pub height: u16,
}

impl Size {
    /// Creates a size.
    pub const fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}

/// An axis-aligned rectangle in terminal coordinates.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Rect {
    /// Top-left position.
    pub min: Point,
    /// Extent of the rectangle.
    pub size: Size,
}

impl Rect {
    /// Creates a rectangle.
    pub const fn new(min: Point, size: Size) -> Self {
        Self { min, size }
    }

    /// Returns whether a point lies inside the rectangle.
    pub fn contains(self, point: Point) -> bool {
        point.x >= self.min.x
            && point.y >= self.min.y
            && point.x < self.min.x + i32::from(self.size.width)
            && point.y < self.min.y + i32::from(self.size.height)
    }
}

/// A terminal color independent of any concrete output protocol.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Color {
    /// Use the terminal's default color.
    Default,
    /// A 256-color palette entry.
    Indexed(u8),
    /// An RGB color.
    Rgb {
        /// Red channel.
        r: u8,
        /// Green channel.
        g: u8,
        /// Blue channel.
        b: u8,
    },
    /// Preserve the color from a lower layer.
    Transparent,
}

/// A single terminal cell with optional glyph and colors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cell {
    /// Character displayed by the cell. `None` leaves the lower layer intact.
    pub glyph: Option<char>,
    /// Foreground color.
    pub foreground: Color,
    /// Background color.
    pub background: Color,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            glyph: Some(' '),
            foreground: Color::Default,
            background: Color::Default,
        }
    }
}

impl Cell {
    /// Creates a cell.
    pub const fn new(character: char) -> Self {
        Self {
            glyph: Some(character),
            foreground: Color::Default,
            background: Color::Default,
        }
    }

    /// Creates a fully transparent cell.
    pub const fn transparent() -> Self {
        Self {
            glyph: None,
            foreground: Color::Transparent,
            background: Color::Transparent,
        }
    }

    /// Sets the foreground color.
    pub const fn foreground(mut self, color: Color) -> Self {
        self.foreground = color;
        self
    }

    /// Sets the background color.
    pub const fn background(mut self, color: Color) -> Self {
        self.background = color;
        self
    }

    /// Returns whether the glyph is transparent.
    pub const fn is_glyph_transparent(self) -> bool {
        self.glyph.is_none()
    }

    /// Returns whether both glyph and colors are transparent.
    pub const fn is_transparent(self) -> bool {
        self.glyph.is_none()
            && matches!(self.foreground, Color::Transparent)
            && matches!(self.background, Color::Transparent)
    }
}

/// A z-order used by render systems.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Layer(pub i16);

/// A terminal frame made of cells.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    size: Size,
    cells: Vec<Cell>,
}

impl Frame {
    /// Creates a cleared frame.
    pub fn new(size: Size) -> Self {
        Self {
            size,
            cells: vec![Cell::default(); usize::from(size.width) * usize::from(size.height)],
        }
    }

    /// Returns the frame size.
    pub const fn size(&self) -> Size {
        self.size
    }

    /// Clears every cell.
    pub fn clear(&mut self) {
        self.cells.fill(Cell::default());
    }

    /// Sets a cell, returning whether the position was inside the frame.
    pub fn set(&mut self, point: Point, cell: Cell) -> bool {
        let Some(index) = self.index(point) else {
            return false;
        };
        self.cells[index] = cell;
        true
    }

    /// Returns a cell at a position.
    pub fn get(&self, point: Point) -> Option<Cell> {
        self.index(point).map(|index| self.cells[index])
    }

    fn index(&self, point: Point) -> Option<usize> {
        if point.x < 0
            || point.y < 0
            || point.x >= i32::from(self.size.width)
            || point.y >= i32::from(self.size.height)
        {
            return None;
        }
        Some(point.y as usize * usize::from(self.size.width) + point.x as usize)
    }
}

mod input;
pub use input::{ButtonState, InputEvent, InputState, Key, KeyCode, KeyboardEvent};

/// The current terminal size resource.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TerminalSize(pub Size);
