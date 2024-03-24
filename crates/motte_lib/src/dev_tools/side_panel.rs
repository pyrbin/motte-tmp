use bevy::{input::common_conditions::input_toggle_active, window::PrimaryWindow};
use bevy_egui::{egui, EguiContext};
use bevy_inspector_egui::bevy_inspector::hierarchy::SelectedEntities;

use super::key_codes;
use crate::{app_state::AppState, prelude::*};

pub struct SidePanelPlugin;

impl Plugin for SidePanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            side_panel_ui
                .run_if(input_toggle_active(true, key_codes::TOGGLE_SIDE_PANEL))
                .run_if(in_state(AppState::InGame)),
        );
    }
}

#[derive(Default, PartialEq, Eq)]
pub(super) enum Panel {
    #[default]
    Hierarchy,
    Resources,
    Assets,
    DebugLayers,
}

pub(super) fn side_panel_ui(
    world: &mut World,
    mut selected_entities: Local<SelectedEntities>,
    mut active_panel: Local<Panel>,
) {
    let mut egui_context = world.query_filtered::<&mut EguiContext, With<PrimaryWindow>>().single(world).clone();

    egui::SidePanel::left("side panel").default_width(350.0).show_separator_line(true).show(
        egui_context.get_mut(),
        |ui| {
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut *active_panel, Panel::Hierarchy, "Hierarchy");
                ui.selectable_value(&mut *active_panel, Panel::Resources, "Resource");
                ui.selectable_value(&mut *active_panel, Panel::Assets, "Assets");
                ui.selectable_value(&mut *active_panel, Panel::DebugLayers, "Debug Layers");
            });

            ui.separator();

            let available_size = ui.available_size();
            let half_height = available_size.y / 2.0;

            let inspector_active = matches!(*active_panel, Panel::Hierarchy) && !selected_entities.is_empty();

            egui::ScrollArea::both()
                .id_source("hierarchy")
                .max_height(if inspector_active { half_height } else { available_size.y })
                .show(ui, |ui| {
                    match *active_panel {
                        Panel::Hierarchy => {
                            bevy_inspector_egui::bevy_inspector::hierarchy::hierarchy_ui(
                                world,
                                ui,
                                &mut selected_entities,
                            );
                        }
                        Panel::Resources => {
                            bevy_inspector_egui::bevy_inspector::ui_for_resources(world, ui);
                        }
                        Panel::Assets => {
                            bevy_inspector_egui::bevy_inspector::ui_for_all_assets(world, ui);
                        }
                        Panel::DebugLayers => {
                            bevy_inspector_egui::bevy_inspector::ui_for_resource::<DebugLayers>(world, ui);
                        }
                    };
                    ui.set_min_width(available_size.x);
                });

            if inspector_active {
                ui.separator();
                ui.add_space(10.0);
                egui::ScrollArea::both().id_source("inspector").max_height(half_height).show(ui, |ui| {
                    match selected_entities.as_slice() {
                        &[entity] => {
                            bevy_inspector_egui::bevy_inspector::ui_for_entity(world, entity, ui);
                        }
                        entities => {
                            bevy_inspector_egui::bevy_inspector::ui_for_entities_shared_components(world, entities, ui);
                        }
                    }
                    ui.set_min_width(available_size.x);
                });
            }
        },
    );
}
