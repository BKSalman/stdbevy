use bevy::prelude::*;

#[cfg(feature = "debug")]
use bevy_inspector_egui::bevy_egui::PrimaryEguiContext;

#[cfg(not(feature = "debug"))]
pub fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

#[cfg(feature = "debug")]
pub fn spawn_camera(
    mut commands: Commands,
    mut egui_global_settings: ResMut<bevy_inspector_egui::bevy_egui::EguiGlobalSettings>,
) {
    egui_global_settings.auto_create_primary_context = false;

    commands.spawn((Camera2d, IsDefaultUiCamera));

    commands.spawn((
        PrimaryEguiContext,
        Camera2d,
        bevy::camera::visibility::RenderLayers::none(),
        Camera {
            order: 1,
            output_mode: bevy::camera::CameraOutputMode::Write {
                blend_state: Some(bevy::render::render_resource::BlendState::ALPHA_BLENDING),
                clear_color: ClearColorConfig::None,
            },
            clear_color: ClearColorConfig::Custom(Color::NONE),
            ..default()
        },
    ));
}

pub struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera);
    }
}
