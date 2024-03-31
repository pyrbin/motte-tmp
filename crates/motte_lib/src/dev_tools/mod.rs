use bevy_egui::EguiPlugin;
use bevy_inspector_egui::DefaultInspectorConfigPlugin;

use crate::{app_state::AppState, asset_management::FontAssets, navigation::agent::Agent, prelude::*};

mod perf_ui;
mod side_panel;

mod key_codes {
    use bevy::input::keyboard::KeyCode;
    pub const TOGGLE_SIDE_PANEL: KeyCode = KeyCode::F1;
    pub const TOGGLE_PERF_PANEL: KeyCode = KeyCode::F2;
}

pub struct DevToolsPlugin;

impl Plugin for DevToolsPlugin {
    fn build(&self, app: &mut App) {
        app_register_types!(AgentDebugLayer);

        app.add_plugins((
            bevy::diagnostic::FrameTimeDiagnosticsPlugin,
            bevy::diagnostic::EntityCountDiagnosticsPlugin,
            bevy::diagnostic::SystemInformationDiagnosticsPlugin,
            bevy::diagnostic::LogDiagnosticsPlugin::filtered(vec![]),
        ));
        if !app.is_plugin_added::<DefaultInspectorConfigPlugin>() {
            app.add_plugins(DefaultInspectorConfigPlugin);
        }
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin);
        }

        app.add_plugins((PhysicsDebugPlugin::default(), bevy_transform_gizmo::TransformGizmoPlugin::default()));

        app.add_plugins((perf_ui::PerfUiPlugin, side_panel::SidePanelPlugin));

        app.insert_gizmo_group(PhysicsGizmos { aabb_color: Some(Color::WHITE), ..default() }, GizmoConfig::default());
        app.init_resource::<DebugLayers>();

        app.add_systems(OnExit(AppState::Loading), semver_ui);
        app.add_systems(
            Update,
            (
                toggle_debug_physics,
                crate::navigation::flow_field::footprint::gizmos.run_if(|d: Res<DebugLayers>| d.debug_footprints),
                crate::navigation::flow_field::layout::gizmos.run_if(|d: Res<DebugLayers>| d.debug_field_layout),
                crate::navigation::flow_field::gizmos_cell_index.run_if(|d: Res<DebugLayers>| d.debug_cell_index),
                crate::navigation::agent::gizmos.run_if(|d: Res<DebugLayers>| d.debug_agents),
                crate::navigation::obstacle::gizmos.run_if(|d: Res<DebugLayers>| d.debug_obstacles),
                crate::navigation::avoidance::gizmos.run_if(|d: Res<DebugLayers>| d.debug_avoidance),
                // TODO: annoying setup, maybe use a macro to generate this :P ?
                crate::navigation::flow_field::fields::obstacle::gizmos::<{ Agent::Huge }>
                    .run_if(|d: Res<DebugLayers>| d.debug_obstacle_field.enabled_for(Agent::Huge)),
                crate::navigation::flow_field::fields::obstacle::gizmos::<{ Agent::Large }>
                    .run_if(|d: Res<DebugLayers>| d.debug_obstacle_field.enabled_for(Agent::Large)),
                crate::navigation::flow_field::fields::obstacle::gizmos::<{ Agent::Medium }>
                    .run_if(|d: Res<DebugLayers>| d.debug_obstacle_field.enabled_for(Agent::Medium)),
                crate::navigation::flow_field::fields::obstacle::gizmos::<{ Agent::Small }>
                    .run_if(|d: Res<DebugLayers>| d.debug_obstacle_field.enabled_for(Agent::Small)),
                crate::navigation::flow_field::fields::flow::gizmos::<{ Agent::Huge }>
                    .run_if(|d: Res<DebugLayers>| d.debug_flow_field.enabled_for(Agent::Huge)),
                crate::navigation::flow_field::fields::flow::gizmos::<{ Agent::Large }>
                    .run_if(|d: Res<DebugLayers>| d.debug_flow_field.enabled_for(Agent::Large)),
                crate::navigation::flow_field::fields::flow::gizmos::<{ Agent::Medium }>
                    .run_if(|d: Res<DebugLayers>| d.debug_flow_field.enabled_for(Agent::Medium)),
                crate::navigation::flow_field::fields::flow::gizmos::<{ Agent::Small }>
                    .run_if(|d: Res<DebugLayers>| d.debug_flow_field.enabled_for(Agent::Small)),
            )
                .run_if(in_state(AppState::InGame)),
        );
    }
}

#[derive(Resource, Reflect)]
pub struct DebugLayers {
    debug_cell_index: bool,
    debug_agents: bool,
    debug_obstacles: bool,
    debug_avoidance: bool,
    debug_footprints: bool,
    debug_obstacle_field: AgentDebugLayer,
    debug_flow_field: AgentDebugLayer,
    debug_field_layout: bool,
    debug_physics: bool,
}

impl Default for DebugLayers {
    fn default() -> Self {
        Self {
            debug_cell_index: true,
            debug_agents: true,
            debug_avoidance: true,
            debug_obstacles: true,
            debug_footprints: true,
            debug_obstacle_field: AgentDebugLayer::Medium,
            debug_flow_field: AgentDebugLayer::Medium,
            debug_field_layout: true,
            debug_physics: true,
        }
    }
}

#[derive(Default, Reflect)]
pub enum AgentDebugLayer {
    #[default]
    Disabled,
    Small = 1,
    Medium = 3,
    Large = 5,
    Huge = 7,
}

impl AgentDebugLayer {
    fn enabled_for(&self, agent: Agent) -> bool {
        if matches!(self, Self::Disabled) {
            return false;
        }
        // TODO: replace with a bit flags
        match agent {
            Agent::Small => matches!(self, Self::Small),
            Agent::Medium => matches!(self, Self::Medium),
            Agent::Large => matches!(self, Self::Large),
            Agent::Huge => matches!(self, Self::Huge),
        }
    }
}

fn toggle_debug_physics(debug_layers: Res<DebugLayers>, mut store: ResMut<GizmoConfigStore>) {
    let (config, _) = store.config_mut::<PhysicsGizmos>();
    config.enabled = debug_layers.debug_physics;
}

#[derive(Component)]
struct SemverUi;

fn semver_ui(mut commands: Commands, assets: Res<FontAssets>) {
    commands
        .spawn((
            Name::ui("semver"),
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(9.0),
                    right: Val::Px(16.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::BLACK.with_a(0.8)),
                ..default()
            },
            SemverUi,
        ))
        .with_children(|builder| {
            builder.spawn((TextBundle::from_sections([TextSection::new(
                crate::version(),
                TextStyle { font: assets.commit_mono_400.clone(), font_size: 16.0, color: Color::WHITE },
            )]),));
        });
}
