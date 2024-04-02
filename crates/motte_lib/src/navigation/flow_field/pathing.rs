use super::{
    cache::FlowFieldCache,
    fields::{
        flow::{Flow, FlowField},
        Cell,
    },
    footprint::Footprint,
    layout::FieldLayout,
    CellIndex,
};
use crate::{
    navigation::agent::{Agent, AgentType, DesiredDirection, TargetDistance},
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

pub(super) fn direction<const AGENT: Agent>(
    mut agents: Query<
        (Entity, &Goal, &mut Flow, &mut DesiredDirection, &mut TargetDistance, &CellIndex),
        With<AgentType<AGENT>>,
    >,
    layout: Res<FieldLayout>,
    flow_field_cache: Res<FlowFieldCache<AGENT>>,
    flow_fields: Query<(&FlowField<AGENT>, Option<Ref<Footprint>>), Without<Disabled<FlowField<AGENT>>>>,
    transforms: Query<Ref<GlobalTransform>>,
) {
    agents.par_iter_mut().for_each(
        |(entity, goal, mut flow, mut desired_direction, mut target_distance, cell_index)| {
            if matches!(goal, Goal::None) {
                *flow = Flow::None;
                **desired_direction = None;
                **target_distance = 0.0;
                return;
            }

            let CellIndex::Valid(cell, index) = cell_index else {
                *flow = Flow::None;
                **desired_direction = None;
                **target_distance = 0.0;
                return;
            };

            let entry = flow_field_cache.get(goal);

            if entry.is_none() {
                *flow = Flow::None;
                **desired_direction = None;
                **target_distance = 0.0;
                return;
            }

            let entry = entry.unwrap();

            unsafe {
                // SAFETY: it's fine :)
                // Pokes the cache timer to keep it alive.
                #[allow(invalid_reference_casting)]
                #[allow(clippy::mut_from_ref)]
                unsafe fn as_mut<T>(reference: &T) -> &mut T {
                    let const_ptr = reference as *const T;
                    let mut_ptr = const_ptr as *mut T;
                    &mut *mut_ptr
                }
                let timer = as_mut(&entry.1);
                timer.reset();
            }

            let (flow_field, footprint) = flow_fields.get(entry.0).unwrap();

            if flow_field.is_empty() {
                *flow = Flow::None;
                **desired_direction = None;
                **target_distance = 0.0;
                return;
            }

            if !flow_field.valid(*cell) {
                *flow = Flow::None;
                **desired_direction = None;
                **target_distance = 0.0;
                return;
            }

            // direction
            let flow_next = flow_field[*index];

            // TODO: maybe move this blending logic to the agent.
            if flow_next.is_repulse() {
                **desired_direction = if let Some(dir) = **desired_direction {
                    const KSI: f32 = 0.100;
                    let direction = dir
                        .xy()
                        .lerp(
                            flow_next.direction().as_direction2d().and_then(|d| d.xy().into()).unwrap_or(Vec2::ZERO),
                            KSI,
                        )
                        .normalize_or_zero();
                    Direction2d::from_xy(direction.x, direction.y).ok()
                } else {
                    flow_next.direction().as_direction2d()
                }
            } else {
                **desired_direction = flow_next.direction().as_direction2d();
            }

            *flow = flow_next;

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
        },
    );
}

pub(super) fn maintain(
    commands: ParallelCommands,
    without_flow: Query<Entity, (With<Goal>, Without<Flow>)>,
    without_goal: Query<Entity, (Without<Goal>, With<Flow>)>,
) {
    without_flow.par_iter().for_each(|entity| {
        commands.command_scope(|mut c| {
            c.entity(entity).insert(Flow::default());
        });
    });

    without_goal.par_iter().for_each(|entity| {
        commands.command_scope(|mut c| {
            c.entity(entity).remove::<Flow>();
        });
    })
}
