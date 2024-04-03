use bevy::{
    ecs::system::{
        lifetimeless::{SQuery, SRes},
        SystemParam,
    },
    input::common_conditions::input_just_pressed,
    render::renderer::RenderAdapterInfo,
};
use iyes_perf_ui::prelude::*;

use super::key_codes;
use crate::{app_state::AppState, asset_management::FontAssets, graphics::pixelate, prelude::*};

pub struct PerfUiPlugin;

impl Plugin for PerfUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(iyes_perf_ui::PerfUiPlugin);
        app.add_perf_ui_entry_type::<PerfUiEntryRenderAdapter>();
        app.add_perf_ui_entry_type::<PerfUiEntryRenderResolution>();
        app.add_systems(OnExit(AppState::Loading), perf_ui);
        app.add_systems(
            Update,
            toggle.run_if(input_just_pressed(key_codes::TOGGLE_PERF_PANEL)).run_if(in_state(AppState::InGame)),
        );
    }
}

#[derive(Component, Default)]
pub struct PerfUiEntryRenderAdapter {
    pub sort_key: i32,
}

impl PerfUiEntry for PerfUiEntryRenderAdapter {
    type Value = String;
    type SystemParam = SRes<RenderAdapterInfo>;

    fn label(&self) -> &str {
        "GPU Adapter"
    }

    fn sort_key(&self) -> i32 {
        self.sort_key
    }

    fn value_color(&self, _value: &Self::Value) -> Option<Color> {
        Color::YELLOW.into()
    }

    fn update_value(
        &self,
        render_adapter_info: &mut <Self::SystemParam as SystemParam>::Item<'_, '_>,
    ) -> Option<Self::Value> {
        let render_adapter_info_name = render_adapter_info.name.clone();
        Some(render_adapter_info_name.to_string())
    }

    fn format_value(&self, value: &Self::Value) -> String {
        value.to_string()
    }
}

#[derive(Component, Default)]
pub struct PerfUiEntryRenderResolution {
    pub sort_key: i32,
}

impl PerfUiEntry for PerfUiEntryRenderResolution {
    type Value = UVec2;
    type SystemParam = SQuery<&'static pixelate::RenderResolution, With<pixelate::Pixelate>>;

    fn label(&self) -> &str {
        "Render Resolution"
    }

    fn sort_key(&self) -> i32 {
        self.sort_key
    }

    fn value_color(&self, _value: &Self::Value) -> Option<Color> {
        Color::YELLOW.into()
    }

    fn update_value(
        &self,
        pixelate_camera: &mut <Self::SystemParam as SystemParam>::Item<'_, '_>,
    ) -> Option<Self::Value> {
        use bevy::ecs::query::QuerySingleError;
        let render_resolution = match pixelate_camera.get_single() {
            Ok(a) => Some(a),
            Err(QuerySingleError::MultipleEntities(_)) => None,
            _ => None,
        };
        render_resolution.map(|a| a.value())
    }

    fn format_value(&self, value: &Self::Value) -> String {
        format!("{}x{}", value.x, value.y)
    }
}

mod sort_keys {
    pub const RENDER_ADAPTER: i32 = 1000;
    pub const WINDOW_RESOLUTION: i32 = 1001;
    pub const RENDER_RESOLUTION: i32 = 1002;
}

fn perf_ui(mut commands: Commands, assets: Res<FontAssets>) {
    commands.spawn((
        Name::ui("perf"),
        PerfUiRoot {
            background_color: Color::BLACK.with_a(0.8),
            font_label: assets.commit_mono_700.clone(),
            font_value: assets.commit_mono_400.clone(),
            font_highlight: assets.commit_mono_700.clone(),
            ..PerfUiRoot::default()
        },
        (
            PerfUiEntryFPS::default(),
            PerfUiEntryFPSWorst::default(),
            PerfUiEntryEntityCount::default(),
            PerfUiEntryCpuUsage::default(),
            PerfUiEntryMemUsage::default(),
            PerfUiEntryRenderAdapter { sort_key: sort_keys::RENDER_ADAPTER },
            PerfUiEntryWindowResolution {
                label: "Window Resolution".into(),
                sort_key: sort_keys::WINDOW_RESOLUTION,
                ..default()
            },
            PerfUiEntryRenderResolution { sort_key: sort_keys::RENDER_RESOLUTION },
        ),
    ));
}

fn toggle(mut perf_ui: Query<&mut Visibility, With<PerfUiRoot>>) {
    let mut visibility = match perf_ui.get_single_mut() {
        Ok(a) => a,
        Err(_) => return,
    };

    match *visibility {
        Visibility::Visible | Visibility::Inherited => *visibility = Visibility::Hidden,
        _ => *visibility = Visibility::Visible,
    }
}
