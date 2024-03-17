use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    simd::{f32x2, f32x4, num::SimdFloat},
};

use ordered_float::NotNan;
use parry2d::na::{SimdComplexField, SimdPartialOrd};

use super::{
    cost::{density::DensityField, obstacle::ObstacleField, CostFields},
    field::{Cell, Direction, Field},
    footprint::Footprint,
    layout::FieldLayout,
    CellIndex,
};
use crate::{navigation::agent::AgentRadius, prelude::*};

macro_rules! notnan {
    ($val:expr) => {
        NotNan::new($val).expect("Value must not be NaN")
    };
}

#[derive(Component, Reflect)]
pub struct FlowField<const R: AgentRadius> {
    potential: Field<f32>,
    #[reflect(ignore)]
    speed: Field<f32x4>,
    #[reflect(ignore)]
    cost: Field<f32x4>,
    #[reflect(ignore)]
    heap: Heap,
}

const SAMPLE_KERNEL: [Direction; 4] = [Direction::East, Direction::West, Direction::North, Direction::South];

impl<const R: AgentRadius> FlowField<R> {
    pub fn from_layout(layout: &FieldLayout) -> Self {
        let size = layout.width() * layout.height();
        Self {
            potential: Field::new(layout.width(), layout.height(), vec![0.0; size]),
            speed: Field::new(layout.width(), layout.height(), vec![f32x4::splat(0.0); size]),
            cost: Field::new(layout.width(), layout.height(), vec![f32x4::splat(0.0); size]),
            heap: Heap::new(layout.width(), layout.height()),
        }
    }

    pub fn build(
        &mut self,
        goals: impl Iterator<Item = Cell>,
        obstacle_field: &ObstacleField,
        density_fields: &DensityField,
    ) {
        self.build_speed_cost(obstacle_field, density_fields);
        self.build_potential(goals, obstacle_field);
    }

    #[inline]
    pub fn sample(&self, local_position_xz: Vec2, speed: f32) -> Vec2 {
        let cell = Cell::floor(local_position_xz.into());
        let delta = (local_position_xz - (cell.as_vec2() + 0.5)).clamp01();
        let inverse_delta = 1.0 - delta;

        let (q11, q12, q21, q22) = std::iter::once(Some(cell))
            .chain(
                self.potential
                    .neighbors_at(cell, [Direction::East, Direction::South, Direction::SouthEast].into_iter()),
            )
            .map(|cell| {
                if let Some(cell) = cell
                    && self.potential[cell] != f32::INFINITY
                {
                    self.sample_velocity(cell, speed).extend(1.0)
                } else {
                    Vec3::ZERO
                }
            })
            .collect_tuple()
            .unwrap();

        let mut weights: f32x4 = default();
        weights[0] = inverse_delta.x * inverse_delta.y;
        weights[1] = inverse_delta.x * delta.y;
        weights[2] = delta.x * inverse_delta.y;
        weights[3] = delta.x * delta.y;

        let interpolated = q11 * weights[0] + q21 * weights[1] + q12 * weights[2] + q22 * weights[3];

        if interpolated.z == 0.0 {
            return Vec2::ZERO;
        }

        interpolated.xy() / interpolated.z
    }

    fn sample_velocity(&self, cell: Cell, speed: f32) -> Vec2 {
        let index = self.potential.index_no_check(cell);
        let speed = f32x4::splat(speed) * self.speed[index];

        let (x, y, z, w) = self
            .potential
            .neighbors_at(cell, SAMPLE_KERNEL.into_iter())
            .map(|cell| if let Some(cell) = cell { self.potential[cell] } else { f32::INFINITY })
            .collect_tuple()
            .unwrap();

        let mask = BVec4A::new(x < f32::INFINITY, y < f32::INFINITY, z < f32::INFINITY, w < f32::INFINITY);
        let potential = Vec4::new(x, y, z, w);
        let mut potential_gradient_anisotropic = Vec4::select(mask, potential - self.potential[index], Vec4::ZERO);

        let potential_gradient = Vec2::new(
            potential_gradient_anisotropic.x - potential_gradient_anisotropic.y,
            potential_gradient_anisotropic.z - potential_gradient_anisotropic.w,
        )
        .abs();
        let normalized_potential_gradient = potential_gradient.normalize_or_zero();

        let mask = BVec2::new(potential_gradient.x > 0.01, potential_gradient.y > 0.01);
        let divider = Vec2::select(mask, normalized_potential_gradient / potential_gradient, Vec2::ONE);
        potential_gradient_anisotropic.x *= divider.x;
        potential_gradient_anisotropic.y *= divider.x;
        potential_gradient_anisotropic.z *= divider.y;
        potential_gradient_anisotropic.w *= divider.y;

        let velocity_anisotropic = -speed * f32x4::from(potential_gradient_anisotropic.to_array());
        Vec2::new(velocity_anisotropic[0] - velocity_anisotropic[1], velocity_anisotropic[2] - velocity_anisotropic[3])
    }

    #[inline]
    fn build_speed_cost(&mut self, obstacle_field: &ObstacleField, density_fields: &DensityField) {
        let width = self.speed.width();
        let height = self.speed.height();
        let avg_velocity_field = density_fields.avg_velocity();
        let density_field = density_fields.density();

        for (i, cell) in (0..width).cartesian_product(0..height).map(|(x, y)| Cell::new(x, y)).enumerate() {
            let mut flow: f32x4 = default();
            let mut density: f32x4 = default();

            for (i, cell) in cell.neighbors_at(SAMPLE_KERNEL.into_iter()).enumerate() {
                let Some(cell) = cell else {
                    continue;
                };

                let Some(index) = self.speed.index(cell) else {
                    continue;
                };

                if obstacle_field[index].traversable(R) {
                    density[i] = density_field[index];
                    flow[i] = avg_velocity_field[index].dot(match i {
                        0 => Vec2::new(R.as_f32(), 0.0),
                        1 => Vec2::new(-1.0 * R.as_f32(), 0.0),
                        2 => Vec2::new(0.0, R.as_f32()),
                        3 => Vec2::new(0.0, -1.0 * R.as_f32()),
                        _ => unreachable!(),
                    })
                }
            }
            const MIN_FLOW: f32 = 0.01;
            flow = flow.simd_max(f32x4::splat(MIN_FLOW)); // Avoid stop or negative speed.

            const MAX_DENSITY: f32 = 1.62;
            const MIN_DENSITY: f32 = 0.32;
            density = (density - f32x4::splat(MIN_DENSITY)).clamp01()
                / (f32x4::splat(MAX_DENSITY) - f32x4::splat(MIN_DENSITY));

            let speed: std::simd::prelude::Simd<f32, 4> = f32x4::splat(1.0) + density * flow;
            self.speed[i] = speed;

            const DISTANCE_WEIGHT: f32 = 0.2;
            const TIME_WEIGHT: f32 = 0.8;
            self.cost[i] = (f32x4::splat(DISTANCE_WEIGHT) * speed + f32x4::splat(TIME_WEIGHT)) / speed;
        }
    }

    #[inline]
    fn build_potential(&mut self, goals: impl Iterator<Item = Cell>, obstacle_field: &ObstacleField) {
        debug_assert!(self.potential.len() == obstacle_field.len());

        let (potential, heap, cost) = (&mut self.potential, &mut self.heap, &self.cost);
        for potent in potential.iter_mut() {
            *potent = f32::INFINITY;
        }

        heap.clear();

        for goal in goals.into_iter() {
            if !obstacle_field.valid(goal) {
                // FIXME: should we panic here?
                continue;
            }
            heap.push(goal, notnan!(0.0));
            potential[goal] = 0.0;
        }

        #[inline]
        fn approx_potential<const R: AgentRadius>(
            cell: Cell,
            potential: &Field<f32>,
            obstacle_field: &ObstacleField,
            cost_field: &Field<f32x4>,
        ) -> f32 {
            let index = cost_field.index_no_check(cell);
            let (east, west, north, south) = obstacle_field
                .neighbors_at(cell, SAMPLE_KERNEL.into_iter())
                .map(|cell| {
                    if let Some(cell) = cell
                        && obstacle_field.traversable(cell, R)
                    {
                        potential[cell]
                    } else {
                        0.0
                    }
                })
                .collect_tuple()
                .unwrap();
            let costs = cost_field[index];
            let cost_east = costs[0];
            let cost_west = costs[1];
            let cost_north = costs[2];
            let cost_south = costs[3];

            let (potential_x, cost_x) =
                if east + cost_east < west + cost_west { (east, cost_east) } else { (west, cost_west) };
            let (potential_y, cost_y) =
                if north + cost_north < south + cost_south { (north, cost_north) } else { (south, cost_south) };

            debug_assert!(potential_x < f32::INFINITY || potential_y < f32::INFINITY);

            #[inline]
            fn solve(a: f32, b: f32) -> f32 {
                let x0 = a - b;
                let x1 = a + b;
                x0.simd_max(x1)
            }

            if potential_x >= f32::INFINITY {
                return solve(potential_y, cost_y);
            }

            if potential_y >= f32::INFINITY {
                return solve(potential_x, cost_x);
            }

            #[inline]
            fn solve_4(a: f32, b: f32, c: f32, d: f32) -> f32 {
                let a2 = a * a;
                let b2 = b * b;
                let c2 = c * c;
                let d2 = d * d;

                let j = -a2 + 2.0 * a * c + b2 - c2 + d2;

                if j < 0.0 {
                    if a < c {
                        return solve(a, b);
                    } else {
                        return solve(c, d);
                    }
                }

                let p0: f32 = j.simd_sqrt() / (b * d);
                let p1 = a / b2 + c / d2;
                let p2 = 1.0 / b2 + 1.0 / d2;

                let x0 = (-p0 + p1) / p2;
                let x1 = (p0 + p1) / p2;

                let r = x0.simd_max(x1);

                if r >= a && r >= c {
                    r
                } else if a < c {
                    solve(a, b)
                } else {
                    solve(c, d)
                }
            }

            solve_4(potential_x, cost_x, potential_y, cost_y)
        }

        while let Some((cell, _)) = heap.pop() {
            for neighbor in obstacle_field.neighbors(cell).filter(|neighbor| obstacle_field.traversable(*neighbor, R)) {
                if heap.contains(neighbor) {
                    continue;
                }

                let potent = approx_potential::<R>(neighbor, potential, obstacle_field, cost);
                if potent < potential[neighbor] {
                    potential[neighbor] = potent;
                    heap.push(neighbor, notnan!(potential[neighbor]));
                }
            }
        }
    }
}

#[derive(Clone, Default)]
struct Heap {
    heap: BinaryHeap<Reverse<(NotNan<f32>, Cell)>>,
    contains: Field<bool>,
}

impl Heap {
    #[inline]
    fn new(width: usize, height: usize) -> Self {
        Self { heap: BinaryHeap::new(), contains: Field::new(width, height, vec![false; width * height]) }
    }
    #[inline]
    fn push(&mut self, cell: Cell, cost: NotNan<f32>) {
        self.heap.push(Reverse((cost, cell)));
        self.contains[cell] = true;
    }

    #[inline]
    fn pop(&mut self) -> Option<(Cell, NotNan<f32>)> {
        let Reverse((cost, cell)) = self.heap.pop()?;
        self.contains[cell] = false;
        Some((cell, cost))
    }
    #[inline]
    fn contains(&self, cell: Cell) -> bool {
        self.contains[cell]
    }
    #[inline]
    fn clear(&mut self) {
        self.heap.clear();
        for cell in self.contains.iter_mut() {
            *cell = false;
        }
    }
}

pub fn update<const R: AgentRadius>(
    cost_fields: Res<CostFields>,
    mut flow_fields: Query<(&mut FlowField<R>, &CellIndex, Option<&Footprint>)>,
) {
    flow_fields.par_iter_mut().for_each(|(mut flow_field, cell_index, footprint)| {
        let goals = match footprint {
            Some(footprint) if let Footprint::Cells(cells) = footprint => cells.iter().cloned().collect_vec(),
            None if let CellIndex::Valid(cell, _) = cell_index => vec![*cell],
            _ => return,
        };
        flow_field.build(goals.into_iter(), &cost_fields.obstacle, &cost_fields.density);
    });
}

#[cfg(feature = "debug")]
pub fn gizmos(mut gizmos: Gizmos, layout: Res<FieldLayout>, flow_fields: Query<&FlowField<{ AgentRadius::Small }>>) {
    for flow_field in &flow_fields {
        for (cell, &cost) in flow_field.potential.iter().enumerate().map(|(i, cost)| (layout.cell_from_index(i), cost))
        {
            let position = layout.position(cell).x0y();
            if cost == f32::INFINITY {
                gizmos.rect(
                    position.y_pad(),
                    Quat::from_rotation_x(PI / 2.),
                    Vec2::ONE * layout.cell_size(),
                    Color::RED,
                );
            }

            let end = position
                + flow_field.sample(layout.transform_point(position.xz()), 100.0).x0y().normalize_or_zero()
                    * (layout.cell_size() * 0.5);
            gizmos.arrow(position + Vec3::Y * 0.1, end + Vec3::Y * 0.1, Color::GREEN);
        }

        // for (index, _) in flow_field.potential.iter().enumerate() {
        //     let cell = layout.cell_from_index(index);
        //     let start = layout.position(cell).x0y();
        //     let end =
        //         start + flow_field.sample(start.xz(), 100.0).x0y().normalize_or_zero() * (layout.cell_size() * 0.75);
        //     gizmos.arrow(start + Vec3::Y * 0.1, end + Vec3::Y * 0.1, Color::GREEN);
        // }
    }
}
