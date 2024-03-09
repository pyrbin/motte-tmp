//! FlowField
use std::sync::{Arc, RwLock};

use self::{
    cache::FlowFieldCache,
    cost::{CellOccupants, CellOccupantsReverse, CostUpdateTasks, DirtyCells},
    field::Cell,
    goal::Goal,
};
use crate::{app_state::AppState, flow_field::cost::OccupancyCells, prelude::*};

pub mod cache;
pub mod cost;
pub mod field;
pub mod flow;
pub mod goal;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FlowFieldSystems {
    Setup,
    Maintain,
    Build,
    Poll,
}

pub struct FlowFieldPlugin;

impl Plugin for FlowFieldPlugin {
    fn build(&self, app: &mut App) {
        app_register_types!(Cell, CellIndex, Goal, FieldLayout, OccupancyCells);

        const DEFAULT_SIZE: usize = 100;
        const DEFAULT_CELL_SIZE: f32 = 2.0;

        app.insert_resource(FieldLayout::default().with_size(DEFAULT_SIZE).with_cell_size(DEFAULT_CELL_SIZE));
        app.insert_resource(DirtyCells::default());
        app.insert_resource(CellOccupants::default());
        app.insert_resource(CellOccupantsReverse::default());
        app.insert_resource(FlowFieldCache::default());
        app.insert_resource(CostUpdateTasks::default());
        app.insert_resource(CostField::new(DEFAULT_SIZE));

        #[cfg(feature = "debug")]
        {
            use bevy_inspector_egui::quick::ResourceInspectorPlugin;
            app.add_plugins(ResourceInspectorPlugin::<FieldLayout>::default());
        }

        app.configure_sets(
            Update,
            (FlowFieldSystems::Setup).after(PhysicsSet::Sync).run_if(in_state(AppState::InGame)),
        );
        app.configure_sets(
            PostUpdate,
            (FlowFieldSystems::Maintain, FlowFieldSystems::Build, FlowFieldSystems::Poll)
                .chain()
                .before(PhysicsSet::Prepare)
                .run_if(in_state(AppState::InGame)),
        );

        app.add_systems(
            Update,
            (
                flow::setup,
                cost::occupancy_cells_setup,
                cost::obstacles_cleanup,
                cost::occupancy_cleanup,
                cache::despawn,
                cache::detect,
                cache::insert,
                goal::setup,
            )
                .in_set(FlowFieldSystems::Setup),
        );

        app.add_systems(
            PostUpdate,
            (
                cell_index,
                cell_index_layout.run_if(resource_exists_and_changed::<FieldLayout>),
                flow::layout_resize.run_if(resource_exists_and_changed::<FieldLayout>),
                flow::moved,
                flow::cost_changed.run_if(resource_exists_and_changed::<CostField>),
                cost::layout_resize.run_if(resource_exists_and_changed::<FieldLayout>),
                cost::occupancy_cells,
                cost::obstacles,
                cache::lifetime,
            )
                .in_set(FlowFieldSystems::Maintain),
        );

        app.add_systems(PostUpdate, (flow::build, cost::update, apply_deferred).in_set(FlowFieldSystems::Build));

        app.add_systems(
            PostUpdate,
            (flow::poll_rebuild_tasks, cost::poll_update_tasks, cache::lifetime, goal::seek)
                .in_set(FlowFieldSystems::Poll),
        );
    }
}

#[derive(Component, Default, Deref, DerefMut)]
pub struct FlowField(Arc<RwLock<flow::FlowField>>);

#[derive(Resource, Default, Deref, DerefMut)]
pub struct CostField(Arc<RwLock<cost::CostField>>);

impl CostField {
    pub fn new(size: usize) -> Self {
        Self(Arc::new(RwLock::new(cost::CostField::new(size))))
    }
}

#[derive(Component, Default, Deref, DerefMut, Reflect)]
pub struct CellIndex(Cell);

fn cell_index(
    mut transforms: Query<(&mut CellIndex, &GlobalTransform), Or<(ChangedPhysicsPosition, Added<CellIndex>)>>,
    field_layout: Res<FieldLayout>,
) {
    transforms.par_iter_mut().for_each(|(mut cell_index, global)| {
        let cell = field_layout.world_to_cell(global.translation());
        if cell_index.0 != cell {
            *cell_index = CellIndex(cell);
        }
    });
}

fn cell_index_layout(mut transforms: Query<(&mut CellIndex, &GlobalTransform)>, field_layout: Res<FieldLayout>) {
    transforms.par_iter_mut().for_each(|(mut cell_index, global)| {
        let cell = field_layout.world_to_cell(global.translation());
        if cell_index.0 != cell {
            *cell_index = CellIndex(cell);
        }
    });
}

#[derive(Default, Reflect)]
pub enum FieldPosition {
    #[default]
    Centered,
    Origin,
    Position(Vec3),
}

#[derive(Resource, Reflect)]
pub struct FieldLayout {
    size: usize,
    cell_size: f32,
    position: FieldPosition,
}

impl FieldLayout {
    pub fn with_size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    pub fn with_cell_size(mut self, cell_size: f32) -> Self {
        self.cell_size = cell_size;
        self
    }
}

impl Default for FieldLayout {
    fn default() -> Self {
        Self { size: 64, cell_size: 1.0, position: FieldPosition::Centered }
    }
}

impl FieldLayout {
    #[inline]
    pub fn world_to_cell(&self, global_position: Vec3) -> Cell {
        let translation = global_position - self.offset();
        Cell::round((translation.x / self.cell_size(), translation.z / self.cell_size()))
    }

    #[inline]
    pub fn cell_to_world(&self, cell: Cell) -> Vec3 {
        let offset = self.offset();
        let world_x = cell.x() as f32 * self.cell_size() + offset.x;
        let world_z = cell.y() as f32 * self.cell_size() + offset.z;
        Vec3::new(world_x, 0.0, world_z)
    }

    #[inline]
    #[allow(unused)]
    pub fn cell_corners(&self, cell: Cell) -> [Vec3; 4] {
        let world_pos = self.cell_to_world(cell);
        let half_size = self.cell_size() / 2.0;
        [
            world_pos + Vec3::new(-half_size, 0.0, -half_size),
            world_pos + Vec3::new(half_size, 0.0, -half_size),
            world_pos + Vec3::new(half_size, 0.0, half_size),
            world_pos + Vec3::new(-half_size, 0.0, half_size),
        ]
    }

    #[inline]
    pub fn in_bounds(&self, cell: Cell) -> bool {
        cell.x() < self.size() && cell.y() < self.size()
    }

    pub fn cell_size(&self) -> f32 {
        self.cell_size.max(f32::EPSILON)
    }

    pub fn size(&self) -> usize {
        self.size.max(1)
    }

    #[inline]
    fn offset(&self) -> Vec3 {
        let half_size = self.cell_size() / 2.0;
        match self.position {
            FieldPosition::Centered => Vec3::new(
                -(self.size() as f32 / 2.0) * self.cell_size() + half_size,
                0.0,
                -(self.size() as f32 / 2.0) * self.cell_size() + half_size,
            ),
            FieldPosition::Origin => Vec3::ZERO,
            FieldPosition::Position(v) => v,
        }
    }

    #[inline]
    pub fn center(&self) -> Vec3 {
        match self.position {
            FieldPosition::Centered => Vec3::ZERO,
            FieldPosition::Origin => Vec3::ZERO,
            FieldPosition::Position(v) => v,
        }
    }
}
