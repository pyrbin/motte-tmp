use super::fields::{self, Cell};
use crate::{navigation::agent::Agent, prelude::*};

pub const CELL_SIZE: fields::Scalar = 1;
pub const CELL_SIZE_F32: f32 = CELL_SIZE as f32;
pub const HALF_CELL_SIZE: f32 = CELL_SIZE_F32 / 2.0;

#[derive(Resource, Clone, Copy, Reflect)]
pub struct FieldLayout {
    width: fields::Scalar,
    height: fields::Scalar,
    offset: Vec2,
}

impl Default for FieldLayout {
    fn default() -> Self {
        const WIDTH: fields::Scalar = 64;
        const HEIGHT: fields::Scalar = 64;
        Self { width: WIDTH, height: HEIGHT, offset: centered_offset(WIDTH, HEIGHT) }
    }
}

impl FieldLayout {
    pub const fn new(width: fields::Scalar, height: fields::Scalar) -> Self {
        let mut layout = Self { width, height, ..Default::default() };
        layout.offset = centered_offset(layout.width, layout.height);
        layout
    }

    #[inline]
    pub const fn width(&self) -> fields::Scalar {
        self.width
    }

    #[inline]
    pub const fn height(&self) -> fields::Scalar {
        self.height
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.width as usize * self.height as usize
    }

    #[inline]
    pub const fn offset(&self) -> Vec2 {
        self.offset
    }

    #[inline]
    pub const fn center(&self) -> Vec2 {
        Vec2::ZERO
    }

    #[inline]
    pub const fn cell(&self, global_position_xz: Vec2) -> Cell {
        let translation = self.transform_point(global_position_xz);
        Cell::round((translation.x / CELL_SIZE_F32, translation.y / CELL_SIZE_F32))
    }

    #[inline]
    pub const fn cell_from_index(&self, index: usize) -> Cell {
        Cell::from_index(index, self.width())
    }

    #[inline]
    pub const fn position(&self, cell: Cell) -> Vec2 {
        let offset = self.offset();
        let world_x = cell.x() as f32 * CELL_SIZE_F32 + offset.x;
        let world_z = cell.y() as f32 * CELL_SIZE_F32 + offset.y;
        Vec2::new(world_x, world_z)
    }

    #[inline]
    pub const fn transform_point(&self, global_position_xz: Vec2) -> Vec2 {
        global_position_xz - self.offset()
    }

    #[inline]
    pub const fn index(&self, cell: Cell) -> Option<usize> {
        if self.valid(cell) {
            Some(cell.index(self.width))
        } else {
            None
        }
    }

    #[inline]
    pub const fn valid(&self, cell: Cell) -> bool {
        cell.x() < self.width && cell.y() < self.height
    }

    #[inline]
    pub fn bounds(&self, agent: Agent) -> impl Iterator<Item = Cell> {
        let width = self.width();
        let height = self.height();
        let offset = agent.radius().ceil() as u8;
        let top_bottom = (0..width).flat_map(move |x| {
            std::iter::once(Cell::new(x, offset - 1)).chain(std::iter::once(Cell::new(x, height - offset)))
        });
        let left_right = (1..height - offset).flat_map(move |y| {
            std::iter::once(Cell::new(offset - 1, y)).chain(std::iter::once(Cell::new(width - offset, y)))
        });

        top_bottom.chain(left_right)
    }
}

#[derive(Resource, Default, Deref, DerefMut, Reflect)]
pub struct FieldBounds<const AGENT: Agent>(Vec<Cell>);

pub(super) fn field_bounds<const AGENT: Agent>(layout: Res<FieldLayout>, mut field_bounds: ResMut<FieldBounds<AGENT>>) {
    if layout.is_changed() || layout.len() != 0 && field_bounds.0.is_empty() {
        let bounds = Agent::ALL.iter().filter(|a| a.radius() <= AGENT.radius()).flat_map(|a| layout.bounds(*a));
        **field_bounds = bounds.collect();
    }
}

#[inline]
const fn centered_offset(width: fields::Scalar, height: fields::Scalar) -> Vec2 {
    Vec2::new(
        -(width as f32 / 2.0) * CELL_SIZE_F32 + HALF_CELL_SIZE,
        -(height as f32 / 2.0) * CELL_SIZE_F32 + HALF_CELL_SIZE,
    )
}

#[cfg(feature = "dev_tools")]
pub(crate) fn gizmos(mut gizmos: Gizmos, layout: Res<FieldLayout>) {
    gizmos.rect(
        layout.center().x0y() + Vec3::Y * 0.1,
        Quat::from_rotation_x(PI / 2.),
        Vec2::new(layout.width() as f32, layout.height() as f32) * CELL_SIZE_F32,
        Color::CYAN,
    );
}
