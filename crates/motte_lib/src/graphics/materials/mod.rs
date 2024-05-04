use bevy::asset::load_internal_asset;

use self::cel::{CelExtension, CelMaterial};
use crate::prelude::*;

pub mod cel;

// TODO: move into a "shader" plugin
const COLORS_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(5569923404675166368);
const EDGES_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(3369923404675556377);
const UTILS_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(5269923424675136362);

pub struct MaterialsPlugin;

impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, UTILS_SHADER_HANDLE, "../../../../../assets/shaders/utils.wgsl", Shader::from_wgsl);
        load_internal_asset!(app, COLORS_SHADER_HANDLE, "../../../../../assets/shaders/colors.wgsl", Shader::from_wgsl);
        load_internal_asset!(app, EDGES_SHADER_HANDLE, "../../../../../assets/shaders/edges.wgsl", Shader::from_wgsl);

        app.add_plugins(MaterialPlugin::<CelMaterial>::default()).register_asset_reflect::<CelMaterial>();

        app.add_systems(PostUpdate, replace_shaders);
    }
}

fn replace_shaders(
    mut commands: Commands,
    query: Query<(Entity, &Handle<StandardMaterial>), With<Handle<StandardMaterial>>>,
    standard_material: ResMut<Assets<StandardMaterial>>,
    mut cel_material: ResMut<Assets<CelMaterial>>,
) {
    for (entity, mat) in &query {
        let Some(mat) = standard_material.get(mat) else {
            continue;
        };

        commands
            .entity(entity)
            .remove::<Handle<StandardMaterial>>()
            .insert(cel_material.add(CelMaterial { base: mat.clone(), extension: CelExtension::default() }));
    }
}
