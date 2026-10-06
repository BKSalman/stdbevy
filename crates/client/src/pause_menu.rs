use bevy::audio::Volume;
use bevy::prelude::*;
use bevy::ui::FocusPolicy;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode};

use crate::module_bindings::*;
use crate::state::*;
use crate::stdb::*;
use crate::ui::*;

const VOLUME_STEP: f32 = 0.1;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States, Reflect)]
pub enum PauseMenu {
    #[default]
    Closed,
    Root,
    Settings,
}

#[cfg_attr(feature = "debug", derive(Reflect))]
#[derive(Component, Debug, Default, Clone)]
pub struct VolumeText;

fn overlay() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            top: px(0),
            bottom: px(0),
            left: px(0),
            right: px(0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        GlobalZIndex(100)
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75))
    }
}

fn panel() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(16),
            padding: px(32),
            border: px(4),
            border_radius: BorderRadius::all(px(12)),
        }
        BorderColor::from(Color::BLACK)
        BackgroundColor(Color::srgb(0.1, 0.1, 0.1))
    }
}

fn spawn_pause_menu(mut commands: Commands) {
    commands
        .spawn_scene(bsn! {
            overlay()
            Children [
                (
                    panel()
                    Children [
                        text("Menu"),
                        (
                            button("Resume")
                            on(|_event: On<Pointer<Press>>, mut next: ResMut<NextState<PauseMenu>>| {
                                next.set(PauseMenu::Closed);
                            })
                        ),
                        (
                            button("Settings")
                            on(|_event: On<Pointer<Press>>, mut next: ResMut<NextState<PauseMenu>>| {
                                next.set(PauseMenu::Settings);
                            })
                        ),
                        (
                            button("Leave game")
                            on(|_event: On<Pointer<Press>>, conn: Res<StdbConn>, state: Res<State<AppState>>, mut next: ResMut<NextState<PauseMenu>>| {
                                let request = match state.get() {
                                    AppState::InLobby => conn.reducers().leave_game(),
                                    AppState::InGame => conn.reducers().withdraw_from_game(),
                                    AppState::MainMenu => return,
                                };
                                if let Err(err) = request {
                                    error!("could not request leaving the game: {err}");
                                }
                                next.set(PauseMenu::Closed);
                            })
                        ),
                        (
                            button("Quit")
                            on(|_event: On<Pointer<Press>>, mut exit: MessageWriter<AppExit>| {
                                exit.write(AppExit::Success);
                            })
                        ),
                    ]
                )
            ]
        })
        .insert((FocusPolicy::Block, DespawnOnExit(PauseMenu::Root)));
}

fn spawn_settings_menu(mut commands: Commands) {
    commands
        .spawn_scene(bsn! {
            overlay()
            Children [
                (
                    panel()
                    Children [
                        text("Settings"),
                        (
                            Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                column_gap: px(12),
                            }
                            Children [
                                (
                                    button("-")
                                    on(|_event: On<Pointer<Press>>, mut volume: ResMut<GlobalVolume>| {
                                        let level = (volume.volume.to_linear() - VOLUME_STEP).max(0.0);
                                        volume.volume = Volume::Linear(level);
                                    })
                                ),
                                (VolumeText),
                                (
                                    button("+")
                                    on(|_event: On<Pointer<Press>>, mut volume: ResMut<GlobalVolume>| {
                                        let level = (volume.volume.to_linear() + VOLUME_STEP).min(1.0);
                                        volume.volume = Volume::Linear(level);
                                    })
                                ),
                            ]
                        ),
                        (
                            button("Toggle fullscreen")
                            on(|_event: On<Pointer<Press>>, mut window: Single<&mut Window, With<PrimaryWindow>>| {
                                window.mode = match window.mode {
                                    WindowMode::Windowed => {
                                        WindowMode::BorderlessFullscreen(MonitorSelection::Current)
                                    }
                                    _ => WindowMode::Windowed,
                                };
                            })
                        ),
                        (
                            button("Back")
                            on(|_event: On<Pointer<Press>>, mut next: ResMut<NextState<PauseMenu>>| {
                                next.set(PauseMenu::Root);
                            })
                        ),
                    ]
                )
            ]
        })
        .insert((FocusPolicy::Block, DespawnOnExit(PauseMenu::Settings)));
}

fn update_volume_text(
    mut commands: Commands,
    volume: Res<GlobalVolume>,
    volume_text: Single<Entity, With<VolumeText>>,
) {
    let percent = (volume.volume.to_linear() * 100.0).round();
    commands
        .entity(*volume_text)
        .insert(Text::new(format!("Volume: {percent}%")));
}

fn toggle_pause_menu(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<PauseMenu>>,
    mut next: ResMut<NextState<PauseMenu>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        next.set(match state.get() {
            PauseMenu::Closed => PauseMenu::Root,
            PauseMenu::Root => PauseMenu::Closed,
            PauseMenu::Settings => PauseMenu::Root,
        });
    }
}

pub struct PauseMenuPlugin;
impl Plugin for PauseMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<PauseMenu>();
        app.add_systems(Update, toggle_pause_menu);
        app.add_systems(OnEnter(PauseMenu::Root), spawn_pause_menu);
        app.add_systems(OnEnter(PauseMenu::Settings), spawn_settings_menu);
        app.add_systems(
            Update,
            update_volume_text.run_if(in_state(PauseMenu::Settings)),
        );
    }
}
