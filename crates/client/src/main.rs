mod cards;
mod module_bindings;
mod stdb;
mod table_list;

use bevy::prelude::*;
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use bevy_stdb::prelude::*;
use spacetimedb_sdk::Identity;

use stdb::*;
use table_list::{TableList, TableListPlugin, ViewListPlugin};

use crate::module_bindings::{
    Game, GameTableAccessor, MyhandTableAccessor, Player, PlayerHand, Seat, SeatTableAccessor,
    create_game, enter_game, gameQueryTableAccess, leave_game, myhand_table,
    myhandQueryTableAccess, play_card, played_cardQueryTableAccess, playerQueryTableAccess,
    seatQueryTableAccess, start_game,
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

#[derive(Component, Debug, Default, Clone)]
pub struct PlayerHandRoot;

#[derive(Component, Debug, Default, Clone)]
pub struct CurrentTurnRoot;

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
        app.add_systems(OnEnter(AppState::InGame), spawn_in_game_ui);

        app.add_plugins((
            TableListPlugin::<Game>::default(),
            TableListPlugin::<Seat>::default(),
            ViewListPlugin::<PlayerHand>::default(),
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

        app.add_systems(
            PreUpdate,
            (update_turn_text).run_if(in_state(AppState::InGame)),
        );

        app.add_systems(
            PreUpdate,
            (handle_game_state,)
                .run_if(resource_exists::<LocalPlayer>.and_eager(resource_exists::<CurrentGame>)),
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
    let current_game = current_game.0;
    commands
        .spawn_scene(bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
            }
            Children [
                (
                    button("Start game")
                    on(move |_event: On<Pointer<Press>>, conn: Res<StdbConn>| {
                        if let Err(err) = conn.reducers().start_game(current_game) {
                            error!("could not request leave_game: {err}");
                        }
                    })
                ),
                (
                    button("Leave game")
                    on(|_event: On<Pointer<Press>>, conn: Res<StdbConn>| {
                        if let Err(err) = conn.reducers().leave_game() {
                            error!("could not request leave_game: {err}");
                        }
                    })
                )
            ]
        })
        .insert(DespawnOnExit(AppState::InLobby));
    commands
        .spawn_scene(players_list(current_game))
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

fn spawn_in_game_ui(mut commands: Commands, current_game: Res<CurrentGame>) {
    let game = current_game.0;
    commands
        .spawn_scene(bsn! {
            Node {
                position_type: PositionType::Absolute,
                left: px(5),
                top: px(5),
            }
            Text::new(format!("current game: {}", game))
        })
        .insert(DespawnOnExit(AppState::InGame));
    commands
        .spawn_scene(bsn! {
            CurrentTurnRoot
            Node {
                position_type: PositionType::Absolute,
                left: px(5),
                top: px(50),
            }
            Text::new(format!("current turn: {}", 0))
        })
        .insert(DespawnOnExit(AppState::InGame));

    commands
        .spawn_scene(bsn! {
            PlayerHandRoot
            Node {
                position_type: PositionType::Absolute,
                bottom: px(0),
                left: px(0),
                right: px(0),
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                padding: px(10),
                column_gap: px(10),
            }
        })
        .insert(DespawnOnExit(AppState::InGame));
}

impl TableList for PlayerHand {
    type Accessor = MyhandTableAccessor;
    type Root = PlayerHandRoot;
    type Order = u64;

    fn order(&self) -> u64 {
        self.seat_id
    }

    fn row(self) -> impl Scene {
        let cards_scenes: Vec<_> = self
            .cards
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let card_name = match c.suit {
                    module_bindings::Suit::Hearts => match c.rank {
                        module_bindings::Rank::Ace => String::from("AceHearts"),
                        module_bindings::Rank::Two => String::from("TwoHearts"),
                        module_bindings::Rank::Three => String::from("ThreeHearts"),
                        module_bindings::Rank::Four => String::from("FourHearts"),
                        module_bindings::Rank::Five => String::from("FiveHearts"),
                        module_bindings::Rank::Six => String::from("SixHearts"),
                        module_bindings::Rank::Seven => String::from("SevenHearts"),
                        module_bindings::Rank::Eight => String::from("EightHearts"),
                        module_bindings::Rank::Nine => String::from("NineHearts"),
                        module_bindings::Rank::Ten => String::from("TenHearts"),
                        module_bindings::Rank::Jack => String::from("JackHearts"),
                        module_bindings::Rank::Queen => String::from("QueenHearts"),
                        module_bindings::Rank::King => String::from("KingHearts"),
                    },
                    module_bindings::Suit::Diamonds => match c.rank {
                        module_bindings::Rank::Ace => String::from("AceDiamonds"),
                        module_bindings::Rank::Two => String::from("TwoDiamonds"),
                        module_bindings::Rank::Three => String::from("ThreeDiamonds"),
                        module_bindings::Rank::Four => String::from("FourDiamonds"),
                        module_bindings::Rank::Five => String::from("FiveDiamonds"),
                        module_bindings::Rank::Six => String::from("SixDiamonds"),
                        module_bindings::Rank::Seven => String::from("SevenDiamonds"),
                        module_bindings::Rank::Eight => String::from("EightDiamonds"),
                        module_bindings::Rank::Nine => String::from("NineDiamonds"),
                        module_bindings::Rank::Ten => String::from("TenDiamonds"),
                        module_bindings::Rank::Jack => String::from("JackDiamonds"),
                        module_bindings::Rank::Queen => String::from("QueenDiamonds"),
                        module_bindings::Rank::King => String::from("KingDiamonds"),
                    },
                    module_bindings::Suit::Clubs => match c.rank {
                        module_bindings::Rank::Ace => String::from("AceClubs"),
                        module_bindings::Rank::Two => String::from("TwoClubs"),
                        module_bindings::Rank::Three => String::from("ThreeClubs"),
                        module_bindings::Rank::Four => String::from("FourClubs"),
                        module_bindings::Rank::Five => String::from("FiveClubs"),
                        module_bindings::Rank::Six => String::from("SixClubs"),
                        module_bindings::Rank::Seven => String::from("SevenClubs"),
                        module_bindings::Rank::Eight => String::from("EightClubs"),
                        module_bindings::Rank::Nine => String::from("NineClubs"),
                        module_bindings::Rank::Ten => String::from("TenClubs"),
                        module_bindings::Rank::Jack => String::from("JackClubs"),
                        module_bindings::Rank::Queen => String::from("QueenClubs"),
                        module_bindings::Rank::King => String::from("KingClubs"),
                    },
                    module_bindings::Suit::Spades => match c.rank {
                        module_bindings::Rank::Ace => String::from("AceSpades"),
                        module_bindings::Rank::Two => String::from("TwoSpades"),
                        module_bindings::Rank::Three => String::from("ThreeSpades"),
                        module_bindings::Rank::Four => String::from("FourSpades"),
                        module_bindings::Rank::Five => String::from("FiveSpades"),
                        module_bindings::Rank::Six => String::from("SixSpades"),
                        module_bindings::Rank::Seven => String::from("SevenSpades"),
                        module_bindings::Rank::Eight => String::from("EightSpades"),
                        module_bindings::Rank::Nine => String::from("NineSpades"),
                        module_bindings::Rank::Ten => String::from("TenSpades"),
                        module_bindings::Rank::Jack => String::from("JackSpades"),
                        module_bindings::Rank::Queen => String::from("QueenSpades"),
                        module_bindings::Rank::King => String::from("KingSpades"),
                    },
                };

                bsn! {
                    button(card_name)
                    on(move |_event: On<Pointer<Press>>, conn: Res<StdbConn>| {
                        if let Err(err) = conn.reducers().play_card(i as u32) {
                            error!("could not request play_card: {err}");
                        }
                    })
                }
            })
            .collect();

        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                column_gap: px(10),
                row_gap: px(10),
            }
            Children [
                { cards_scenes }
            ]
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
        subs.subscribe_query(SubKey::Game, |q| {
            q.from.game().r#where(|g| g.id.eq(game_id))
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
        subs.subscribe_query(SubKey::Game, |q| q.from.game());

        commands.remove_resource::<CurrentGame>();
        commands.set_state(AppState::MainMenu);
    }
}

fn handle_game_state(
    mut commands: Commands,
    mut current_game: ResMut<CurrentGame>,
    mut subs: ResMut<StdbSubs>,
    mut games: ReadUpdateMessage<Game>,
) {
    for msg in games.read() {
        if msg.new.id != current_game.0 {
            continue;
        }

        let new_game_state = (msg.new.state != msg.old.state).then_some(msg.new.state);

        if let Some(new_game_state) = new_game_state {
            match new_game_state {
                module_bindings::GameState::Lobby => {
                    let game_id = msg.new.id;
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
                    subs.subscribe_query(SubKey::Game, |q| {
                        q.from.game().r#where(|g| g.id.eq(game_id))
                    });

                    commands.set_state(AppState::InLobby);
                }
                module_bindings::GameState::Playing => {
                    let game_id = msg.new.id;
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
                    subs.subscribe_query(SubKey::Game, |q| {
                        q.from.game().r#where(|g| g.id.eq(game_id))
                    });

                    commands.set_state(AppState::InGame);
                }
                module_bindings::GameState::Ended => {
                    subs.unsubscribe(&SubKey::Seat).ok();
                    subs.unsubscribe(&SubKey::PlayedCard).ok();
                    // XXX: is this needed?
                    subs.unsubscribe(&SubKey::PlayerHand).ok();
                    subs.unsubscribe(&SubKey::Game).ok();

                    commands.set_state(AppState::MainMenu);
                }
            }
        }
    }
}

fn update_turn_text(
    mut games: ReadUpdateMessage<Game>,
    mut text: Single<&mut Text, With<CurrentTurnRoot>>,
) {
    for msg in games.read() {
        text.0 = format!("current turn: {}", msg.new.current_seat);
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
