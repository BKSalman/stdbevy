mod cards;
mod module_bindings;
mod stdb;
mod table_list;

use bevy::prelude::*;
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use bevy_stdb::prelude::*;
use spacetimedb_sdk::Identity;

use stdb::*;
use table_list::{TableList, TableListPlugin};

use crate::module_bindings::{
    Game, GameTableAccessor, Player, Seat, SeatTableAccessor, create_game, enter_game,
    gameQueryTableAccess, leave_game, myhandQueryTableAccess, played_cardQueryTableAccess,
    playerQueryTableAccess, seatQueryTableAccess,
};

#[derive(Component, Debug, Default)]
pub struct PlayerMarker(Identity);

#[derive(Component, Debug, Default)]
pub struct SeatId(u64);

#[derive(Resource, Debug, Default, Clone)]
pub struct LocalPlayer(Identity);

#[derive(Component, Debug, Default)]
pub struct NetTransform {
    x: f32,
    y: f32,
}

#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct CurrentGame(u64);

#[derive(Component, Debug, Default, Clone)]
pub struct GamesListRoot;

#[derive(Component, Debug, Default, Clone)]
pub struct PlayersListRoot;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    MainMenu,
    InLobby,
    InGame,
}

fn main() -> AppExit {
    App::new().add_plugins(AppPlugin).run()
}

pub struct AppPlugin;
impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Window {
                    title: String::from("SpacetimeDB + Bevy template"),
                    fit_canvas_to_parent: true,
                    ..default()
                }
                .into(),
                ..default()
            }),
        )
        .add_plugins(EguiPlugin::default())
        .add_plugins(WorldInspectorPlugin::new());

        app.init_state::<AppState>();

        app.add_plugins(MyStdbPlugin);

        app.add_systems(Startup, spawn_camera);

        app.add_systems(OnEnter(AppState::MainMenu), spawn_main_menu_ui);
        app.add_systems(OnEnter(AppState::InLobby), spawn_in_lobby_ui);

        app.add_plugins((
            TableListPlugin::<Game>::default(),
            TableListPlugin::<Seat>::default(),
        ));

        app.add_systems(
            PreUpdate,
            (subscribe_on_connect, despawn_player).run_if(resource_exists::<StdbConn>),
        );

        app.add_systems(
            PreUpdate,
            (
                spawn_player,
                enter_lobby_when_seated,
                exit_lobby_when_unseated,
            )
                .run_if(resource_exists::<LocalPlayer>),
        );
    }
}

fn button(label: impl Into<String>) -> impl Scene {
    let label: String = label.into();
    bsn! {
        Button
        Node {
            padding: px(10),
            border: px(5),
            border_radius: BorderRadius::MAX,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        BorderColor::from(Color::BLACK)
        BackgroundColor(Color::srgb(0.15, 0.15, 0.15))
        Children [(
            Text(label)
            TextColor(Color::srgb(0.9, 0.9, 0.9))
            TextShadow
        )]
        on(|event: On<Pointer<Enter>>, mut commands: Commands| {
            commands.entity(event.entity).insert(
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15).lighter(0.1))
            );
        })
        on(|event: On<Pointer<Leave>>, mut commands: Commands| {
            commands.entity(event.entity).insert(
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15))
            );
        })
        on(|event: On<Pointer<Press>>, mut commands: Commands| {
            commands.entity(event.entity).insert(
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15).lighter(0.2))
            );
        })
        on(|event: On<Pointer<Release>>, mut commands: Commands| {
            commands.entity(event.entity).insert(
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15).lighter(0.1))
            );
        })
    }
}

fn spawn_in_lobby_ui(mut commands: Commands, current_game: Res<CurrentGame>) {
    commands
        .spawn_scene(bsn! {
            button("Leave game")
            on(|_event: On<Pointer<Press>>, conn: Res<StdbConn>| {
                if let Err(err) = conn.reducers().leave_game() {
                    error!("could not request leave_game: {err}");
                }
            })
        })
        .insert(DespawnOnExit(AppState::InLobby));
    commands
        .spawn_scene(players_list(current_game.0))
        .insert(DespawnOnExit(AppState::InLobby));
}

fn players_list(game_id: u64) -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            top: px(80),
            row_gap: px(10),
        }
        Children [
            Text::new(format!("game: #{game_id}")),
            (
                PlayersListRoot
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(10),
                }
            ),
        ]
    }
}

impl TableList for Seat {
    type Accessor = SeatTableAccessor;
    type Root = PlayersListRoot;
    type Order = u64;

    fn order(&self) -> u64 {
        self.id
    }

    fn row(self) -> impl Scene {
        let seat_id = self.id;
        bsn! {
            Node {
                flex_direction: FlexDirection::Column,
            }
            Children [
                Text::new(format!("seat: #{seat_id}")),
            ]
        }
    }
}

fn spawn_main_menu_ui(mut commands: Commands) {
    commands
        .spawn_scene(create_game_ui())
        .insert(DespawnOnExit(AppState::MainMenu));
    commands
        .spawn_scene(games_list_ui())
        .insert(DespawnOnExit(AppState::MainMenu));
}

fn create_game_ui() -> impl Scene {
    bsn! {
        (
            button("Create a game")
            on(|_event: On<Pointer<Press>>, conn: Res<StdbConn>| {
                if let Err(err) = conn.reducers().create_game() {
                    error!("could not request create_game: {err}");
                }
            })
        )
    }
}

fn games_list_ui() -> impl Scene {
    bsn! {
        GamesListRoot
        Node {
            flex_direction: FlexDirection::Column,
            top: px(80),
            row_gap: px(10),
        }
    }
}

impl TableList for Game {
    type Accessor = GameTableAccessor;
    type Root = GamesListRoot;
    type Order = u64;

    fn order(&self) -> u64 {
        self.id
    }

    fn row(self) -> impl Scene {
        let game_id = self.id;
        bsn! {
            (
                button(format!("Join game #{game_id}"))
                on(move |_event: On<Pointer<Press>>, conn: Res<StdbConn>| {
                    if let Err(err) = conn.reducers().enter_game(game_id) {
                        error!("could not request enter_game: {err}");
                    }
                })
            )
        }
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn subscribe_on_connect(
    mut commands: Commands,
    mut connected_msgs: ReadStdbConnectedMessage,
    mut subs: ResMut<StdbSubs>,
) {
    for msg in connected_msgs.read() {
        info!("connected as {:?}", msg.identity);
        commands.insert_resource(LocalPlayer(msg.identity));
        subs.subscribe_query(SubKey::Player, |q| q.from.player());
        subs.subscribe_query(SubKey::Game, |q| q.from.game());

        let me = msg.identity;
        subs.subscribe_query(SubKey::MySeat, move |q| {
            q.from.seat().r#where(|s| s.player_id.eq(me))
        });
    }
}

fn enter_lobby_when_seated(
    mut commands: Commands,
    local: Res<LocalPlayer>,
    mut subs: ResMut<StdbSubs>,
    mut seats: ReadInsertMessage<Seat>,
) {
    for msg in seats.read() {
        if msg.row.player_id != local.0 {
            continue;
        }

        let game_id = msg.row.game_id;
        subs.subscribe_query(SubKey::Seat, move |q| {
            q.from.seat().r#where(|s| s.game_id.eq(game_id))
        });
        subs.subscribe_query(SubKey::PlayedCard, move |q| {
            q.from.played_card().r#where(|pc| pc.game_id.eq(game_id))
        });
        // XXX: is this needed?
        subs.subscribe_query(SubKey::PlayerHand, move |q| {
            q.from.myhand().r#where(|myhand| myhand.game_id.eq(game_id))
        });

        commands.insert_resource(CurrentGame(game_id));
        commands.set_state(AppState::InLobby);
    }
}

fn exit_lobby_when_unseated(
    mut commands: Commands,
    local: Res<LocalPlayer>,
    mut subs: ResMut<StdbSubs>,
    mut seats: ReadDeleteMessage<Seat>,
) {
    for msg in seats.read() {
        if msg.row.player_id != local.0 {
            continue;
        }

        subs.unsubscribe(&SubKey::Seat).ok();
        subs.unsubscribe(&SubKey::PlayedCard).ok();
        subs.unsubscribe(&SubKey::PlayerHand).ok();

        commands.remove_resource::<CurrentGame>();
        commands.set_state(AppState::MainMenu);
    }
}

fn spawn_player(
    mut commands: Commands,
    local: Res<LocalPlayer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut insert_player_msgs: ReadInsertMessage<module_bindings::Seat>,
) {
    for msg in insert_player_msgs.read() {
        commands.spawn((PlayerMarker(msg.row.player_id), SeatId(msg.row.id)));
    }
}

fn despawn_player(
    mut commands: Commands,
    players: Query<(Entity, &PlayerMarker)>,
    mut delete_msgs: ReadDeleteMessage<Player>,
) {
    for msg in delete_msgs.read() {
        for (entity, marker) in &players {
            if marker.0 == msg.row.identity {
                commands.entity(entity).despawn();
            }
        }
    }
}
