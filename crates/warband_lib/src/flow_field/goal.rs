use super::{cache::FlowFieldCache, field::Cell, CellIndex, FieldLayout, FlowField};
use crate::{
    navigation::agent::{Seek, TargetDistance},
    prelude::*,
};
// TODO: Handle multi cell goal (Cells(SmallVec<[Cell; 8]>)) & Entity OccupancyCells.
#[derive(Component, Clone, Copy, Default, PartialEq, Eq, Ord, PartialOrd, Hash, Debug, From, Reflect)]
#[reflect(Component)]
pub enum Goal {
    #[default]
    None,
    Entity(Entity),
    Cell(Cell),
}

pub fn setup(mut commands: Commands, agents: Query<Entity, (With<Goal>, Without<Seek>)>) {
    for entity in &agents {
        commands.entity(entity).insert(Seek(None)).insert(TargetDistance::default());
    }
}

pub fn seek(
    mut agents: Query<(Entity, &Goal, &mut Seek, &mut TargetDistance, &CellIndex)>,
    field_layout: Res<FieldLayout>,
    flow_field_cache: Res<FlowFieldCache>,
    flow_fields: Query<&FlowField, Without<Deactivated<FlowField>>>,
    transforms: Query<Ref<GlobalTransform>>,
) {
    agents.par_iter_mut().for_each(|(entity, goal, mut seek, mut target_distance, cell_index)| {
        if matches!(goal, Goal::None) {
            *seek = Seek(None);
            **target_distance = 0.0;
            return;
        }

        let entry = flow_field_cache.get(goal);

        if entry.is_none() {
            *seek = Seek(None);
            **target_distance = 0.0;
            return;
        }

        let entry = entry.unwrap();
        unsafe {
            #[allow(invalid_reference_casting)]
            unsafe fn as_mut<T>(reference: &T) -> &mut T {
                let const_ptr = reference as *const T;
                let mut_ptr = const_ptr as *mut T;
                &mut *mut_ptr
            }
            let timer = as_mut(&entry.ttl);
            timer.reset();
        }

        let flow_field = flow_fields.get(entry.field).unwrap();

        let Ok(flow_field) = flow_field.read() else {
            *seek = Seek(None);
            **target_distance = 0.0;
            return;
        };

        if flow_field.empty() {
            *seek = Seek(None);
            **target_distance = 0.0;
            return;
        }

        if !flow_field.in_bounds(**cell_index) {
            *seek = Seek(None);
            **target_distance = 0.0;
            return;
        }

        let direction = flow_field[**cell_index];
        *seek = Seek(direction.as_direction2d());

        let transform = transforms.get(entity).unwrap();
        let position = transform.translation().xz();
        match (goal, transform.is_changed()) {
            (Goal::Cell(cell), true) => {
                **target_distance = position.distance(field_layout.cell_to_world(*cell).xz());
            }
            (Goal::Entity(entity), _) => {
                let goal_transform = transforms.get(*entity).unwrap();
                if goal_transform.is_changed() || transform.is_changed() {
                    let goal_position = goal_transform.translation().xz();
                    **target_distance = position.distance(goal_position);
                }
            }
            _ => (),
        }
    });
}
