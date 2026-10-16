use bevy::prelude::*;

#[cfg(feature = "debug")]
use bevy_inspector_egui::{
    DefaultInspectorConfigPlugin,
    bevy_egui::{EguiContext, EguiPlugin, EguiPrimaryContextPass, PrimaryEguiContext},
    bevy_inspector::{ui_for_state, ui_for_world},
    egui,
};


#[cfg(debug_assertions)]
use bevy::input::common_conditions::input_just_pressed;

#[cfg(debug_assertions)]
use crate::pause_menu::PauseMenu;
#[cfg(debug_assertions)]
use crate::stdb::StdbConn;
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

/// Despawns everything scoped to `S`'s current value and re-runs that value's
/// `OnEnter` schedule, so the UI it spawns is rebuilt from the current code.
#[cfg(debug_assertions)]
fn restart_state<S: States>(world: &mut World) {
    let Some(state) = world.get_resource::<State<S>>().map(|s| s.get().clone()) else {
        return;
    };

    let scoped: Vec<Entity> = world
        .query::<(Entity, &DespawnOnExit<S>)>()
        .iter(world)
        .filter(|(_, scope)| scope.0 == state)
        .map(|(entity, _)| entity)
        .collect();

    for entity in scoped {
        // A scoped entity can be the child of another one, and so already gone.
        if let Ok(entity) = world.get_entity_mut(entity) {
            entity.despawn();
        }
    }

    // States with nothing registered on entry (`PauseMenu::Closed`, say) have no
    // such schedule; that is not an error here.
    let _ = world.try_run_schedule(OnEnter(state));
}

/// Rebuilds the UI of every active state in place, without leaving the state.
#[cfg(debug_assertions)]
pub fn respawn_state_ui(world: &mut World) {
    restart_state::<AppState>(world);
    restart_state::<RoundState>(world);
    restart_state::<PauseMenu>(world);
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
                respawn_state_ui.run_if(input_just_pressed(KeyCode::F6)),
            ),
        );

        // Systems and observers are patched in place, but scenes that are
        // already spawned are not re-resolved — so rebuild them on every patch,
        // which is what makes editing a `spawn_*_ui` function show up live.
        #[cfg(all(debug_assertions, feature = "hotpatching"))]
        app.add_systems(
            Update,
            respawn_state_ui.run_if(bevy::ecs::schedule::common_conditions::on_message::<
                bevy::ecs::HotPatched,
            >),
        );

        let _ = app;
    }
}
