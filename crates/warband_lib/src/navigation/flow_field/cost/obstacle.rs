use crate::{
    navigation::{
        agent::AgentRadius,
        flow_field::{
            field::{Cell, Field},
            footprint::Footprint,
            layout::FieldLayout,
        },
        obstacle::Obstacle,
    },
    prelude::*,
};

#[derive(Clone, Reflect, Deref, DerefMut)]
pub struct ObstacleField(Field<Cost>);

impl ObstacleField {
    pub fn from_layout(field_layout: &FieldLayout) -> Self {
        Self(Field::new(field_layout.width(), field_layout.height(), vec![default(); field_layout.len()]))
    }

    #[inline]
    pub fn update(&mut self, footprint: &[Cell], cost: Cost) {
        for &cell in footprint {
            if !self.valid(cell) {
                continue;
            }
            self[cell] = cost
        }
    }

    pub fn traversable(&self, cell: Cell, agent_radius: AgentRadius) -> bool {
        self[cell].traversable(agent_radius)
    }

    pub fn clear(&mut self) {
        for i in 0..self.len() {
            self[i] = Cost::default()
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Reflect)]
#[repr(u8)]
pub enum Cost {
    Blocked,
    Traversable(AgentRadius),
}

impl Default for Cost {
    fn default() -> Self {
        Cost::Traversable(AgentRadius::LARGEST)
    }
}

impl Cost {
    pub fn traversable(&self, agent_radius: AgentRadius) -> bool {
        matches!(self, Cost::Traversable(radius) if *radius >= agent_radius)
    }
}

pub fn update(mut cost_fields: ResMut<super::CostFields>, obstacles: Query<&Footprint, With<Obstacle>>) {
    let obstacle_field = &mut cost_fields.obstacle;

    obstacle_field.clear();

    for footprint in &obstacles {
        if footprint.empty() {
            continue;
        }

        #[inline]
        fn expansion_from_radius(radius: AgentRadius) -> usize {
            (radius as usize as f32 / 2.0 - 0.5) as usize
        }

        for (radius, cells) in AgentRadius::largest_to_smallest()
            .into_iter()
            .skip(1)
            .map(|r| (r, footprint.expand(expansion_from_radius(r))))
            .filter_map(|(r, f)| f.map(|f| (r, f)))
        {
            let cells: Vec<_> = cells.collect();
            obstacle_field.update(cells.as_slice(), Cost::Traversable(radius));
        }
    }

    for footprint in &obstacles {
        if let Footprint::Cells(cells) = footprint {
            obstacle_field.update(cells, Cost::Blocked);
        }
    }
}

#[cfg(feature = "debug")]
pub(crate) fn gizmos(mut gizmos: Gizmos, layout: Res<FieldLayout>, cost_fields: Res<super::CostFields>) {
    let obstacle_field = &cost_fields.obstacle;

    for (cell, cost) in obstacle_field.iter().enumerate().map(|(i, cost)| (layout.cell_from_index(i), cost)) {
        let position = layout.position(cell).x0y();
        let color = match cost {
            Cost::Blocked => Color::RED,
            Cost::Traversable(radius) if radius == &AgentRadius::LARGEST => Color::NONE,
            Cost::Traversable(radius) => Color::RED.with_a(
                1.0 - ((*radius as u8 as f32 - AgentRadius::SMALLEST as u8 as f32)
                    / (AgentRadius::LARGEST as u8 as f32 / AgentRadius::SMALLEST as u8 as f32)),
            ),
        };
        gizmos.rect(position.y_pad(), Quat::from_rotation_x(PI / 2.), Vec2::ONE / 1.5 * layout.cell_size(), color);
    }
}
