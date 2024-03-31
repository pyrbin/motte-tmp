#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::Cursor;

use bevy::{
    prelude::*,
    render::RenderPlugin,
    window::{PresentMode, PrimaryWindow, WindowPlugin},
    winit::WinitWindows,
};
use bevy_embedded_assets::{EmbeddedAssetPlugin, PluginMode};

pub fn name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

pub fn main() {
    #[cfg(all(debug_assertions, target_arch = "wasm32"))]
    console_error_panic_hook::set_once();

    let mut app = App::new();

    let default_plugins = DefaultPlugins
        .set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("{} {}", name(), motte_lib::version()),
                present_mode: PresentMode::AutoNoVsync,
                resolution: (1280., 720.).into(),
                #[cfg(debug_assertions)]
                position: WindowPosition::Centered(MonitorSelection::Index(1)),
                ..default()
            }),
            ..default()
        })
        .set(RenderPlugin {
            // note: https://github.com/bevyengine/bevy/issues/9975
            render_creation: bevy::render::settings::RenderCreation::Automatic(bevy::render::settings::WgpuSettings {
                backends: Some(bevy::render::settings::Backends::VULKAN),
                ..default()
            }),
            ..default()
        });

    app.add_plugins(
        default_plugins
            .build()
            .add_before::<bevy::asset::AssetPlugin, _>(EmbeddedAssetPlugin { mode: PluginMode::ReplaceDefault }),
    );

    app.add_plugins(motte_lib::Plugin);

    #[cfg(not(target_arch = "wasm32"))]
    app.add_systems(Startup, set_window_icon);

    app.run();
}

#[cfg(not(target_arch = "wasm32"))]
fn set_window_icon(windows: NonSend<WinitWindows>, primary_window: Query<Entity, With<PrimaryWindow>>) {
    let primary_entity = primary_window.single();
    let primary = windows.get_window(primary_entity).unwrap();
    let icon_buf = Cursor::new(include_bytes!("../../../assets/images/bevy.png"));
    if let Ok(image) = image::load(icon_buf, image::ImageFormat::Png) {
        let image = image.into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        let icon = winit::window::Icon::from_rgba(rgba, width, height).unwrap();
        primary.set_window_icon(Some(icon));
    };
}
