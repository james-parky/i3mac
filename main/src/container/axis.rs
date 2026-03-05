use core_graphics::Direction;
use std::fmt::Display;

#[derive(Debug, Default, Copy, Clone, Hash, PartialEq)]
pub enum Axis {
    Vertical,
    #[default]
    Horizontal,
}

impl Display for Axis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Vertical => write!(f, "Vertical"),
            Self::Horizontal => write!(f, "Horizontal"),
        }
    }
}

impl Axis {
    pub(crate) fn can_resize_in_direction(&self, direction: Direction) -> bool {
        matches!(
            (self, direction),
            (Axis::Horizontal, Direction::Left)
                | (Axis::Horizontal, Direction::Right)
                | (Axis::Vertical, Direction::Up)
                | (Axis::Vertical, Direction::Down)
        )
    }
}
