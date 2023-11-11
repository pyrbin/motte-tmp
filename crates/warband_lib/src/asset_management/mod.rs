use bevy::prelude::*;
use bevy_asset_loader::{asset_collection::AssetCollection, loading_state::LoadingStateAppExt, prelude::LoadingState};

use crate::{app_state::AppState, prelude::*};

pub struct AssetManagementPlugin;

impl Plugin for AssetManagementPlugin {
    fn build(&self, app: &mut App) {
        app.add_loading_state(LoadingState::new(AppState::Loading).continue_to_state(AppState::InGame))
            .add_collection_to_loading_state::<_, FontAssets>(AppState::Loading)
            .add_collection_to_loading_state::<_, GlbAssets>(AppState::Loading)
            .add_collection_to_loading_state::<_, ImageAssets>(AppState::Loading);
    }
}

#[derive(AssetCollection, Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct FontAssets {
    #[asset(path = "fonts/iAWriterQuattroS-Regular.ttf")]
    pub ia_writer_quattro: Handle<Font>,

    #[asset(path = "fonts/CommitMono-400-Regular.otf")]
    pub commit_mono_400: Handle<Font>,

    #[asset(path = "fonts/CommitMono-700-Regular.otf")]
    pub commit_mono_700: Handle<Font>,
}

#[derive(AssetCollection, Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct GlbAssets {
    #[asset(path = "glb/monkey.glb#Scene0")]
    pub monkey: Handle<Scene>,

    #[asset(path = "glb/fox.glb#Scene0")]
    pub fox: Handle<Scene>,

    #[asset(path = "glb/frog.glb#Scene0")]
    pub frog: Handle<Scene>,

    #[asset(path = "glb/ramp.glb#Scene0")]
    pub ramp: Handle<Scene>,
}

#[derive(AssetCollection, Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct ImageAssets {
    #[asset(path = "images/bevy.png")]
    pub bevy: Handle<Image>,

    #[asset(path = "images/proto_dark.png")]
    pub proto_dark: Handle<Image>,
}
