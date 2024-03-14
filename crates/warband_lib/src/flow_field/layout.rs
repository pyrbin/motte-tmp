use super::field::Cell;
use crate::prelude::*;

#[derive(Default, Reflect)]
pub enum FieldPosition {
    #[default]
    Centered,
    Origin,
    Position(Vec2),
}

#[derive(Resource, Reflect)]
pub struct FieldLayout {
    width: usize,
    height: usize,
    cell_size: f32,
    position: FieldPosition,
}

impl FieldLayout {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height, ..Default::default() }
    }

    pub fn with_cell_size(mut self, cell_size: f32) -> Self {
        self.cell_size = cell_size;
        self
    }
}

impl Default for FieldLayout {
    fn default() -> Self {
        Self { width: 64, height: 64, cell_size: 1.0, position: FieldPosition::Centered }
    }
}

impl FieldLayout {
    #[inline]
    pub fn cell(&self, global_position_xz: Vec2) -> Cell {
        let translation = global_position_xz - self.offset();
        Cell::round((translation.x / self.cell_size(), translation.y / self.cell_size()))
    }

    #[inline]
    pub fn cell_from_index(&self, index: usize) -> Cell {
        Cell::from_index(index, self.width())
    }

    #[inline]
    pub fn position(&self, cell: Cell) -> Vec2 {
        let offset = self.offset();
        let world_x = cell.x() as f32 * self.cell_size() + offset.x;
        let world_z = cell.y() as f32 * self.cell_size() + offset.y;
        Vec2::new(world_x, world_z)
    }

    #[inline]
    pub fn index(&self, cell: Cell) -> Option<usize> {
        if self.valid(cell) {
            Some(cell.index(self.width))
        } else {
            None
        }
    }

    #[inline]
    pub fn valid(&self, cell: Cell) -> bool {
        cell.x() < self.width && cell.y() < self.height
    }

    pub fn cell_size(&self) -> f32 {
        self.cell_size.max(f32::EPSILON)
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.height
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.width * self.height
    }

    #[inline]
    fn offset(&self) -> Vec2 {
        let half_size = self.cell_size() / 2.0;
        match self.position {
            FieldPosition::Centered => Vec2::new(
                -(self.width() as f32 / 2.0) * self.cell_size() + half_size,
                -(self.height() as f32 / 2.0) * self.cell_size() + half_size,
            ),
            FieldPosition::Origin => Vec2::ZERO,
            FieldPosition::Position(v) => v,
        }
    }

    #[inline]
    pub fn center(&self) -> Vec2 {
        match self.position {
            FieldPosition::Centered => Vec2::ZERO,
            FieldPosition::Origin => Vec2::ZERO,
            FieldPosition::Position(v) => v,
        }
    }
}

#[cfg(feature = "debug")]
pub(crate) fn gizmos(mut gizmos: Gizmos, layout: Res<FieldLayout>) {
    gizmos.rect(
        layout.center().x0y() + Vec3::Y * 0.1,
        Quat::from_rotation_x(PI / 2.),
        Vec2::new(layout.width() as f32, layout.height() as f32) * layout.cell_size(),
        Color::CYAN,
    );
}
