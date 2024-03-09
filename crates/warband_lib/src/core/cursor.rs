use bevy::{
    input::{mouse::MouseButtonInput, ButtonState},
    window::PrimaryWindow,
};

use crate::{app_state::AppState, prelude::*};

const DRAGGING_THRESHOLD: f32 = 0.02;

pub struct CursorPlugin;
impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app_register_types!(CursorDrag, CursorClick, CursorDoubleClick);

        app.add_event::<CursorClick>();
        app.add_event::<CursorDoubleClick>();
        app.add_event::<CursorDrag>();
        app.insert_resource(CursorButtonState::default());
        app.insert_resource(CursorPosition::default());
        app.add_systems(
            Update,
            (
                update_position,
                update_dragging.run_if(resource_exists_and_changed::<CursorPosition>),
                update_button_input,
                double_click,
            )
                .chain()
                .run_if(in_state(AppState::InGame)),
        );
    }
}

#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub(crate) struct CursorPosition {
    position: Vec2,
    ndc: Vec2,
}

impl CursorPosition {
    #[allow(unused)]
    pub fn position(&self) -> Vec2 {
        self.position
    }
    #[allow(unused)]
    pub fn ndc(&self) -> Vec2 {
        self.ndc
    }
}

#[derive(Resource, Default, Clone, Deref, DerefMut)]
pub struct CursorButtonState(micromap::Map<MouseButton, DragState, 4>);

#[derive(Debug, Clone)]
pub enum DragState {
    Pressed { ndc: Vec2 },
    Dragging { start_ndc: Vec2, current_ndc: Vec2 },
}

#[derive(Event, Debug, Clone, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
pub enum CursorDrag {
    Moved { button: MouseButton, start_ndc: Vec2, current_ndc: Vec2 },
    Released { button: MouseButton, start_ndc: Vec2, end_ndc: Vec2 },
}

#[derive(Event, Debug, Clone, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
pub struct CursorClick {
    pub button: MouseButton,
    pub ndc: Vec2,
}

#[derive(Event, Debug, Clone, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
pub struct CursorDoubleClick {
    pub button: MouseButton,
    pub ndc: Vec2,
}

fn update_position(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut cursor_pos: ResMut<CursorPosition>,
    mut cursor_moved_events: EventReader<CursorMoved>,
) {
    if let Some(last_mouse_position) = cursor_moved_events.read().last() {
        cursor_pos.position = last_mouse_position.position;
        if let Ok(window) = windows.get(last_mouse_position.window) {
            let window_size = Vec2::new(window.physical_width() as f32, window.physical_height() as f32);
            cursor_pos.ndc = 2.0 * (last_mouse_position.position / window_size) - 1.0;
        }
    }
}

fn update_dragging(
    cursor_pos: Res<CursorPosition>,
    mut cursor_button_state: ResMut<CursorButtonState>,
    mut drags: EventWriter<CursorDrag>,
) {
    for (button, drag_state) in cursor_button_state.0.iter_mut() {
        match drag_state {
            DragState::Pressed { ndc } => {
                if cursor_pos.ndc.distance(*ndc) >= DRAGGING_THRESHOLD {
                    let start_ndc = *ndc;
                    *drag_state = DragState::Dragging { start_ndc, current_ndc: cursor_pos.ndc };
                    drags.send(CursorDrag::Moved { button: *button, start_ndc, current_ndc: cursor_pos.ndc });
                }
            }
            DragState::Dragging { start_ndc, current_ndc } => {
                *current_ndc = cursor_pos.ndc;
                drags.send(CursorDrag::Moved { button: *button, start_ndc: *start_ndc, current_ndc: cursor_pos.ndc });
            }
        }
    }
}

fn update_button_input(
    cursor_pos: Res<CursorPosition>,
    mut cursor_button_state: ResMut<CursorButtonState>,
    mut input_events: EventReader<MouseButtonInput>,
    mut drags: EventWriter<CursorDrag>,
    mut click: EventWriter<CursorClick>,
) {
    for event in input_events.read() {
        match event.state {
            ButtonState::Released => {
                if let Some((_, drag_state)) = cursor_button_state.0.remove_entry(&event.button) {
                    match drag_state {
                        DragState::Pressed { ndc } => {
                            click.send(CursorClick { button: event.button, ndc });
                        }
                        DragState::Dragging { start_ndc, current_ndc } => {
                            drags.send(CursorDrag::Released { button: event.button, start_ndc, end_ndc: current_ndc });
                        }
                    }
                }
            }
            ButtonState::Pressed => {
                cursor_button_state.0.insert(event.button, DragState::Pressed { ndc: cursor_pos.ndc });
            }
        }
    }
}

fn double_click(
    mut clicks: EventReader<CursorClick>,
    mut double_clicks: EventWriter<CursorDoubleClick>,
    mut last_click_position: Local<Option<Vec2>>,
    mut last_click_time: Local<f64>,
    time: Res<Time>,
) {
    const DOUBLE_CLICK_THRESHOLD: f64 = 0.5;
    for cursor_click in clicks.read() {
        let current_time = time.elapsed_seconds_f64();
        if last_click_position.map_or(true, |p| p.distance(cursor_click.ndc) < DRAGGING_THRESHOLD)
            && (current_time - *last_click_time) < DOUBLE_CLICK_THRESHOLD
        {
            double_clicks.send(CursorDoubleClick { button: cursor_click.button, ndc: cursor_click.ndc });
        }

        *last_click_time = time.elapsed_seconds_f64();
        *last_click_position = Some(cursor_click.ndc);
    }
}
