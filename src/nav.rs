use bevy::{
    input::{
        mouse::{MouseButtonInput, MouseScrollUnit, MouseWheel},
        ButtonState,
    },
    prelude::*,
    window::PrimaryWindow,
};
use bevy_egui::EguiContext;

pub fn nav_plugin(app: &mut App) {
    app.add_event::<Pick>();
    app.add_systems(PreUpdate, pick);
    app.add_systems(
        Update,
        (
            zoom_camera,
            toggle_pan,
            pan_camera.run_if(resource_exists::<LastDrag>),
        ),
    );
}

#[derive(Resource)]
struct LastDrag(Vec2);

fn pan_camera(
    camera: Query<&Camera>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut panner: Query<&mut Transform, With<Camera>>,
    mut last_drag: ResMut<LastDrag>,
) {
    let pos = windows.single().cursor_position().unwrap();
    let pos = camera
        .single()
        .viewport_to_world(&GlobalTransform::IDENTITY, pos)
        .unwrap();
    let delta = pos.origin.xy() - last_drag.0;
    last_drag.0 = pos.origin.xy();

    let mut panner = panner.single_mut();
    panner.translation -= delta.extend(0.0);
}

fn toggle_pan(
    mut clicks: EventReader<MouseButtonInput>,
    mut commands: Commands,
    camera: Query<&Camera>,
    windows: Query<&Window>,
) {
    for click in clicks.read().filter(|x| x.button == MouseButton::Right) {
        let window = windows.get(click.window).unwrap();
        let pos = window.cursor_position().unwrap();
        let pos = camera
            .single()
            .viewport_to_world(&GlobalTransform::IDENTITY, pos)
            .unwrap();
        if click.state == ButtonState::Pressed {
            commands.insert_resource(LastDrag(pos.origin.xy()));
        } else {
            commands.remove_resource::<LastDrag>();
        }
    }
}

pub fn egui_blocking(ctx: &EguiContext) -> bool {
    ctx.get().is_pointer_over_area() || ctx.get().is_using_pointer()
}

fn zoom_camera(
    ui: Query<&EguiContext, With<Window>>,
    mut camera: Query<&mut Projection>,
    mut scrolls: EventReader<MouseWheel>,
) {
    if egui_blocking(ui.single()) {
        scrolls.clear();
        return;
    }
    let mut projection = camera.single_mut();
    for scroll in scrolls.read() {
        let base = 1.0
            / match scroll.unit {
                MouseScrollUnit::Line => 1.1f32,
                MouseScrollUnit::Pixel => 1.01f32,
            };
        let amount = base.powf(scroll.y);
        if let Projection::Orthographic(projection) = projection.as_mut() {
            projection.scale *= amount;
        }
    }
}

#[derive(Event, Debug)]
pub struct Pick {
    pub down: Vec2,
    pub up: Vec2,
}

fn pick(
    ui: Query<&EguiContext, With<Window>>,
    camera: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    mut clicks: EventReader<MouseButtonInput>,
    mut picks: EventWriter<Pick>,
    mut last_down: Local<Option<Vec2>>,
) {
    if egui_blocking(ui.single()) {
        clicks.clear();
        return;
    }
    let (camera, camera_transform) = camera.single();
    for click in clicks.read().filter(|x| x.button == MouseButton::Left) {
        let window = windows.get(click.window).unwrap();
        let cursor = window.cursor_position().unwrap();
        let pick = camera
            .viewport_to_world_2d(camera_transform, cursor)
            .unwrap();
        match click.state {
            ButtonState::Pressed => {
                *last_down = Some(pick);
            }
            ButtonState::Released => {
                if let Some(down) = *last_down {
                    picks.send(Pick { down, up: pick });
                    *last_down = None;
                }
            }
        }
    }
}
