use bevy::prelude::*;

#[cfg(feature = "debug")]
use bevy_inspector_egui::{
    DefaultInspectorConfigPlugin,
    bevy_egui::{EguiContext, EguiPlugin, EguiPrimaryContextPass, PrimaryEguiContext},
    bevy_inspector::{ui_for_state, ui_for_world},
    egui,
};


#[cfg(debug_assertions)]
#[cfg(debug_assertions)]
use crate::stdb::StdbConn;
#[cfg(debug_assertions)]
use crate::game::{bidding::spawn_players_biddings, spawn_in_game_ui};
#[cfg(debug_assertions)]
use crate::lobby::spawn_in_lobby_ui;
#[cfg(debug_assertions)]
use crate::menu::spawn_main_menu_ui;
#[cfg(any(feature = "debug", debug_assertions))]
use crate::state::*;

#[cfg(feature = "debug")]
pub fn inspector_panel(world: &mut World) {
    let egui_context = world
        .query_filtered::<&mut EguiContext, With<PrimaryEguiContext>>()
        .single(world);

    let Ok(egui_context) = egui_context else {
        return;
    };
    let mut egui_context = egui_context.clone();
    let ctx = egui_context.get_mut().clone();

    let window = world
        .query_filtered::<&Window, With<bevy::window::PrimaryWindow>>()
        .single(world);

    let Ok(window) = window else {
        return;
    };
    let scale_factor = window.scale_factor();
    let physical_size = UVec2::new(window.physical_width(), window.physical_height());

    let mut viewport_ui = egui::Ui::new(
        ctx.clone(),
        "inspector_viewport".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );

    let right = egui::Panel::right("inspector")
        .default_size(150.0)
        .resizable(true)
        .show_inside(&mut viewport_ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("AppState");
                ui.push_id("app_state", |ui| ui_for_state::<AppState>(world, ui));
                ui.separator();
                ui.heading("RoundState");
                ui.push_id("round_state", |ui| ui_for_state::<RoundState>(world, ui));
                ui.separator();
                ui.collapsing("World", |ui| ui_for_world(world, ui));
            });
        })
        .response
        .rect
        .width();

    let occluded = UVec2::new((right * scale_factor) as u32, 0);
    let size = physical_size.saturating_sub(occluded);

    let mut cameras = world.query_filtered::<&mut Camera, Without<EguiContext>>();
    let Ok(mut camera) = cameras.single_mut(world) else {
        return;
    };

    camera.viewport = (size.x > 0 && size.y > 0).then(|| bevy::camera::Viewport {
        physical_position: UVec2::ZERO,
        physical_size: size,
        ..default()
    });
}

#[cfg(debug_assertions)]
pub fn dev_start_solo_hotkey(keys: Res<ButtonInput<KeyCode>>, conn: Res<StdbConn>) {
    use crate::module_bindings::dev_start_solo;

    if keys.just_pressed(KeyCode::F5)
        && let Err(err) = conn.reducers().dev_start_solo()
    {
        error!("could not request dev_start_solo: {err}");
    }
}

#[cfg(debug_assertions)]
pub fn dev_respawn_ui_hotkey(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    scoped: Query<(Entity, &DespawnOnExit<AppState>)>,
    state: Res<State<AppState>>,
) {
    if !keys.just_pressed(KeyCode::F6) {
        return;
    }

    for (entity, scope) in &scoped {
        if scope.0 == *state.get() {
            commands.entity(entity).despawn();
        }
    }

    match state.get() {
        AppState::MainMenu => commands.run_system_cached(spawn_main_menu_ui),
        AppState::InLobby => commands.run_system_cached(spawn_in_lobby_ui),
        AppState::InGame => {
            commands.run_system_cached(spawn_in_game_ui);
            commands.run_system_cached(spawn_players_biddings);
        }
    }
}

pub struct DevPlugin;
impl Plugin for DevPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "debug")]
        app.add_plugins(EguiPlugin::default())
            .add_plugins(DefaultInspectorConfigPlugin)
            .add_systems(EguiPrimaryContextPass, inspector_panel);

        #[cfg(debug_assertions)]
        app.add_systems(
            Update,
            (
                dev_start_solo_hotkey.run_if(resource_exists::<StdbConn>),
                dev_respawn_ui_hotkey,
            ),
        );

        let _ = app;
    }
}
