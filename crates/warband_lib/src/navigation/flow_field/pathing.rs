use super::{
    cache::FlowFieldCache,
    fields::{flow::FlowField, Cell, Direction},
    footprint::Footprint,
    layout::FieldLayout,
    CellIndex,
};
use crate::{
    navigation::agent::{Agent, AgentType, Seek, TargetDistance, TargetReached},
    prelude::*,
};

#[derive(Component, Clone, Copy, Default, PartialEq, Eq, Ord, PartialOrd, Hash, Debug, From, Reflect)]
#[reflect(Component)]
pub enum Goal {
    #[default]
    None,
    Entity(Entity),
    Cell(Cell),
}

pub(super) fn seek<const AGENT: Agent>(
    mut agents: Query<(Entity, &Goal, &mut Seek, &mut TargetDistance, &CellIndex), With<AgentType<AGENT>>>,
    layout: Res<FieldLayout>,
    flow_field_cache: Res<FlowFieldCache<AGENT>>,
    flow_fields: Query<(&FlowField<AGENT>, Option<Ref<Footprint>>), Without<Disabled<FlowField<AGENT>>>>,
    transforms: Query<Ref<GlobalTransform>>,
) {
    agents.par_iter_mut().for_each(|(entity, goal, mut seek, mut target_distance, cell_index)| {
        if matches!(goal, Goal::None) {
            *seek = Seek(None);
            **target_distance = 0.0;
            return;
        }

        let CellIndex::Valid(cell, index) = cell_index else {
            *seek = Seek(None);
            **target_distance = 0.0;
            return;
        };

        let entry = flow_field_cache.get(goal);

        if entry.is_none() {
            *seek = Seek(None);
            **target_distance = 0.0;
            return;
        }

        let entry = entry.unwrap();

        unsafe {
            // SAFETY: it's fine :)
            #[allow(invalid_reference_casting)]
            unsafe fn as_mut<T>(reference: &T) -> &mut T {
                let const_ptr = reference as *const T;
                let mut_ptr = const_ptr as *mut T;
                &mut *mut_ptr
            }
            let timer = as_mut(&entry.1);
            timer.reset();
        }

        let (flow_field, footprint) = flow_fields.get(entry.0).unwrap();

        if flow_field.empty() {
            *seek = Seek(None);
            **target_distance = 0.0;
            return;
        }

        if !flow_field.valid(*cell) {
            *seek = Seek(None);
            **target_distance = 0.0;
            return;
        }

        // direction
        let direction = flow_field[*index];
        *seek = Seek(direction.as_direction2d());

        // distance
        let transform = transforms.get(entity).unwrap();
        let position = transform.translation().xz();
        match (goal, transform.is_changed()) {
            (Goal::Cell(cell), true) => {
                **target_distance = position.distance(layout.position(*cell));
            }
            (Goal::Entity(entity), _) => {
                if let Some(footprint) = footprint
                    && let Some(cells) = footprint.cells()
                {
                    if footprint.is_changed() || transform.is_changed() {
                        // perf: introduce static shapes for Obstacles & Footprints with static radius
                        **target_distance = cells
                            .iter()
                            .map(|&c| position.distance(layout.position(c)))
                            .min_by(|a, b| a.partial_cmp(b).expect("Tried to compare a NaN"))
                            .unwrap_or(f32::MAX);
                    }
                } else {
                    let goal = transforms.get(*entity).unwrap();
                    if goal.is_changed() || transform.is_changed() {
                        **target_distance = position.distance(goal.translation().xz());
                    }
                }
            }
            _ => (),
        }
    });
}
