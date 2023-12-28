use std::sync::{Arc, RwLock};

use bevy::{
    ecs::system::{lifetimeless::Read, SystemParam},
    tasks::{AsyncComputeTaskPool, Task},
    transform::TransformSystem,
};
use futures_lite::future;
use oxidized_navigation::{query, tiles::NavMeshTiles, NavMesh, NavMeshSettings, OxidizedNavigationPlugin};

use crate::{app_state::AppState, prelude::*};

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PathingSystems {
    Pathing,
    Poll,
}

pub struct PathingPlugin;

impl Plugin for PathingPlugin {
    fn build(&self, app: &mut App) {
        app_register_types!(Path, PathTarget);

        app.configure_sets(
            PostUpdate,
            (PathingSystems::Pathing, PathingSystems::Poll)
                .run_if(in_state(AppState::InGame))
                .after(TransformSystem::TransformPropagate),
        );

        // TODO: use bevy_pathmesh once updated.
        let cell_width = 0.5;
        let navmesh_settings = NavMeshSettings {
            cell_width,
            cell_height: cell_width / 2.0,
            tile_width: 100,
            world_half_extents: 1000.0,
            world_bottom_bound: -100.0,
            max_traversable_slope_radians: (40.0_f32 - 0.1).to_radians(),
            walkable_height: 20,
            walkable_radius: 2,
            step_height: 3,
            min_region_area: 100,
            merge_region_area: 500,
            max_contour_simplification_error: 1.1,
            max_edge_length: 80,
            max_tile_generation_tasks: Some(128),
        };
        app.add_plugins(OxidizedNavigationPlugin::<Collider>::new(navmesh_settings));

        app.add_systems(
            PostUpdate,
            (path_target, sync_path_target_transform, cleanup_path_target).chain().in_set(PathingSystems::Pathing),
        );

        app.add_systems(PostUpdate, (async_poll_path).in_set(PathingSystems::Poll));
    }
}

#[derive(SystemParam)]
pub struct Pathfinder<'w, 's> {
    pub nav_mesh: Res<'w, NavMesh>,
    pub nav_mesh_settings: Res<'w, NavMeshSettings>,
    pub commands: Commands<'w, 's>,
    pub transform_query: Query<'w, 's, Read<Transform>>,
}

impl<'w, 's> Pathfinder<'w, 's> {
    pub fn compute_path(&mut self, entity: Entity, end: Vec3) {
        let target_transform = self.transform_query.get(entity).expect("target entity should have transform");
        // TODO(?): raycast to destination, if no hits, skip compute path.
        self.commands.entity(entity).insert(ComputePath::new(
            self.nav_mesh.get(),
            self.nav_mesh_settings.clone(),
            target_transform.translation,
            end,
        ));
    }
}

#[derive(Component, Debug, Clone, Copy, Reflect)]
pub enum PathTarget {
    Position(Vec3),
    Entity(Entity),
}

#[derive(Component, Default, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct Path(pub(super) Vec<Vec3>);

impl Path {
    pub(crate) fn new(waypoints: Vec<Vec3>) -> Self {
        debug_assert!(waypoints.len() >= 2);
        Self(waypoints)
    }

    pub fn current(&self) -> Option<&Vec3> {
        self.last()
    }

    pub fn end(&self) -> Vec3 {
        self[0]
    }
}

#[derive(Component)]
struct ComputePath(Task<Vec<Vec3>>);

impl ComputePath {
    fn new(tiles: Arc<RwLock<NavMeshTiles>>, settings: NavMeshSettings, start: Vec3, end: Vec3) -> Self {
        let thread_pool = AsyncComputeTaskPool::get();
        let task = thread_pool.spawn(async move {
            let tiles = tiles.read().expect("tiles should be readable");

            match query::find_path(&tiles, &settings, start, end, None, None) {
                Ok(path) => path,
                Err(err) => {
                    error!("error with pathfinding: {:?}", err);
                    Vec::new()
                }
            }
        });

        Self(task)
    }
}

fn async_poll_path(mut commands: Commands, mut compute_paths: Query<(Entity, &mut ComputePath)>) {
    for (entity, mut compute_path) in &mut compute_paths {
        if let Some(mut path) = future::block_on(future::poll_once(&mut compute_path.0)) {
            let mut entity_commands = commands.entity(entity);
            if path.len() >= 2 {
                path.reverse();
                entity_commands.insert(Path::new(path));
            };
            entity_commands.remove::<ComputePath>();
        }
    }
}

fn path_target(
    mut pathfinder: Pathfinder,
    mut path_targets: Query<(Entity, &PathTarget), Changed<PathTarget>>,
    transform: Query<&GlobalTransform>,
) {
    for (entity, path_target) in &mut path_targets {
        match path_target {
            PathTarget::Position(position) => {
                pathfinder.compute_path(entity, *position);
            }
            PathTarget::Entity(target_entity) => {
                let target_transform = transform.get(*target_entity).expect("target entity should have transform");
                pathfinder.compute_path(entity, target_transform.translation());
            }
        }
    }
}

fn sync_path_target_transform(
    mut pathfinder: Pathfinder,
    path_targets: Query<(Entity, &PathTarget)>,
    transform: Query<Ref<GlobalTransform>>,
) {
    for (entity, target_entity) in &mut path_targets.iter().filter_map(|(entity, path_target)| match path_target {
        PathTarget::Entity(target_entity) => Some((entity, target_entity)),
        _ => None,
    }) {
        let target_transform = transform.get(*target_entity).expect("target entity should have transform");
        if target_transform.is_changed() {
            continue;
        }

        pathfinder.compute_path(entity, target_transform.translation());
    }
}

fn cleanup_path_target(mut commands: Commands, mut removed_path_target: RemovedComponents<PathTarget>) {
    for entity in &mut removed_path_target.read() {
        if let Some(mut commands) = commands.get_entity(entity) {
            commands.remove::<(Path, ComputePath)>();
        }
    }
}
