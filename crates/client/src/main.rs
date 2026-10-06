mod cards;
mod module_bindings;
mod stdb;
mod table_list;

use bevy::{
    color::palettes::{css::DARK_GREY, tailwind::SLATE_300},
    input::keyboard::KeyboardInput,
    input_focus::FocusedInput,
    prelude::*,
    text::{EditableText, EditableTextFilter, TextCursorStyle},
};

#[cfg(feature = "debug")]
use bevy_inspector_egui::{
    DefaultInspectorConfigPlugin,
    bevy_egui::{EguiContext, EguiPlugin, EguiPrimaryContextPass, PrimaryEguiContext},
    bevy_inspector::{ui_for_state, ui_for_world},
    egui,
};

use bevy_stdb::prelude::*;
use spacetimedb_sdk::{Identity, table::TableLike};

use stdb::*;
use table_list::{TableList, TableListPlugin, ViewListPlugin};

use crate::module_bindings::{
    AcceptedOffer, Game, GameTableAccess, GameTableAccessor, MyhandTableAccess,
    MyhandTableAccessor, NegotiationOffer, PlacedBid, Player, PlayerHand, RejectedOffer, Seat,
    SeatTableAccess, SeatTableAccessor, accept_negotiation, accepted_offerQueryTableAccess,
    create_game, enter_game, gameQueryTableAccess, leave_game, myhandQueryTableAccess,
    negotiate_a_fold, negotiation_offerQueryTableAccess, place_bid, placed_bidQueryTableAccess,
    played_cardQueryTableAccess, playerQueryTableAccess, reject_negotiation,
    rejected_offerQueryTableAccess, seatQueryTableAccess, start_game, start_showdown,
    withdraw_from_bidding, withdraw_from_game,
};

#[derive(Component, Debug, Default, Clone)]
pub struct PlayerMarker(Identity);

#[cfg_attr(feature = "debug", derive(Reflect))]
#[derive(Component, Debug, Default, Clone)]
pub struct SeatId(u64);

#[cfg_attr(feature = "debug", derive(Reflect))]
#[derive(Component, Debug, Default, Clone)]
pub struct PlayerPosition(u8);

#[derive(Resource, Debug, Default, Clone)]
pub struct LocalPlayer(Identity);

#[cfg_attr(feature = "debug", derive(Reflect))]
#[derive(Resource, Debug, Default)]
pub struct LocalSeat(u64);

#[cfg_attr(feature = "debug", derive(Reflect))]
#[derive(Resource, Debug, Default)]
pub struct LocalPosition(u8);

#[cfg_attr(feature = "debug", derive(Reflect))]
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct CurrentGame {
    id: u64,
    player_count: u8,
}

#[cfg_attr(feature = "debug", derive(Reflect))]
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct Shaikh(u64);

#[cfg_attr(feature = "debug", derive(Reflect))]
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct RoundWinner(u64);

#[derive(Component, Debug, Default, Clone)]
pub struct GamesListRoot;

#[derive(Component, Debug, Default, Clone)]
pub struct PlayersListRoot;

#[derive(Component, Debug, Default, Clone)]
pub struct PlayerHandRoot;

#[derive(Component, Debug, Default, Clone)]
pub struct CurrentTurnRoot;

#[derive(Component, Debug, Default, Clone)]
pub struct BidTextInput;

#[derive(Component, Debug, Default, Clone)]
pub struct BiddingContainer;

#[derive(Component, Debug, Default, Clone)]
pub struct NegotiationTextInput;

#[derive(Component, Debug, Default, Clone)]
pub struct LocalBidText;

#[derive(Component, Debug, Default, Clone)]
pub struct OnlineBidText;

#[derive(Component, Debug, Default, Clone)]
pub struct HandScoreText;

#[derive(Component, Debug, Default, Clone)]
pub struct NegotiationPrompt;

#[derive(Component, Debug, Default, Clone)]
pub struct ReceivedNegotiationOffer;

#[derive(Component, Debug, Default, Clone)]
pub struct RejectedNegotiationOffer;

#[derive(Component, Debug, Default, Clone)]
pub struct AcceptedNegotiationOffer;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States, Reflect)]
enum AppState {
    #[default]
    MainMenu,
    InLobby,
    InGame,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, SubStates, Reflect)]
#[source(AppState = AppState::InGame)]
enum RoundState {
    #[default]
    Dealing,
    Bidding,
    Negotiation,
    Showdown,
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
        );

        #[cfg(feature = "debug")]
        app.add_plugins(EguiPlugin::default())
            .add_plugins(DefaultInspectorConfigPlugin)
            .add_systems(EguiPrimaryContextPass, inspector_panel);

        app.init_state::<AppState>().add_sub_state::<RoundState>();

        app.add_plugins(MyStdbPlugin);

        app.add_systems(Startup, spawn_camera);

        app.add_systems(OnEnter(AppState::MainMenu), spawn_main_menu_ui);
        app.add_systems(OnEnter(AppState::InLobby), spawn_in_lobby_ui);
        app.add_systems(OnEnter(AppState::InGame), spawn_in_game_ui);
        app.add_systems(
            OnEnter(RoundState::Bidding),
            (spawn_bidding_ui, spawn_players_biddings),
        );
        app.add_systems(OnEnter(RoundState::Negotiation), spawn_negotiation_ui);
        app.add_systems(OnEnter(RoundState::Showdown), spawn_showdown_ui);

        app.add_plugins((
            TableListPlugin::<Game>::default(),
            TableListPlugin::<Seat>::default(),
            ViewListPlugin::<PlayerHand>::default(),
        ));

        app.add_systems(
            Update,
            (subscribe_on_connect, despawn_player).run_if(resource_exists::<StdbConn>),
        );

        app.add_systems(
            PreUpdate,
            (enter_lobby_when_seated, exit_lobby_when_unseated)
                .after(StdbSet::Flush)
                .run_if(resource_exists::<LocalPlayer>),
        );

        app.add_systems(
            PreUpdate,
            (handle_game_state).after(StdbSet::Flush).run_if(
                resource_exists::<LocalPlayer>
                    .and_eager(resource_exists::<CurrentGame>)
                    .and_eager(resource_exists::<LocalSeat>)
                    .and_eager(resource_exists::<LocalPosition>),
            ),
        );

        app.add_systems(
            Update,
            (
                update_turn_text,
                update_hand_score_text,
                handle_new_round,
                handle_placed_bids,
            )
                .run_if(in_state(AppState::InGame)),
        );

        app.add_systems(
            Update,
            (new_negotiation, negotiation_accepted, negotiation_rejected)
                .run_if(in_state(RoundState::Negotiation)),
        );

        #[cfg(debug_assertions)]
        app.add_systems(
            Update,
            (
                dev_start_solo_hotkey.run_if(resource_exists::<StdbConn>),
                dev_respawn_ui_hotkey,
            ),
        );
    }
}

#[cfg(feature = "debug")]
fn inspector_panel(world: &mut World) {
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

fn text(content: impl Into<String>) -> impl Scene {
    let content: String = content.into();
    bsn! {
        Text::new(content)
        TextFont {
            font: bevy::text::FontSourceTemplate::Family("Noto Naskh Arabic")
        }
    }
}

fn spawn_in_lobby_ui(mut commands: Commands, current_game: Res<CurrentGame>) {
    let current_game = current_game.id;
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
        .spawn_scene(bsn! {
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    width: percent(100.0),
                    height: percent(100.0),
                    position_type: PositionType::Absolute,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                }
                Children [
                    (
                        button("Create a game")
                        on(|_event: On<Pointer<Press>>, conn: Res<StdbConn>| {
                            if let Err(err) = conn.reducers().create_game() {
                                error!("could not request create_game: {err}");
                            }
                        })
                    ),
                    (
                        GamesListRoot
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: px(10),
                        }
                    )
                ]
            )
        })
        .insert(DespawnOnExit(AppState::MainMenu));
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

fn spawn_in_game_ui(
    mut commands: Commands,
    current_game: Res<CurrentGame>,
    local_seat: Res<LocalSeat>,
) {
    let game = current_game.id;
    let local_seat = local_seat.0;
    commands
        .spawn_scene(bsn! {
            Node {
                position_type: PositionType::Absolute,
                left: px(5),
                top: px(5),
            }
            Children [
                (
                    Text::new(format!("current game: {}", game))
                    Text::new(format!("your seat: {}", local_seat))
                ),
                (
                    button("withdraw")
                    on(move |mut event: On<Pointer<Press>>, conn: Res<StdbConn>| {
                        if let Err(err) = conn.reducers().withdraw_from_game() {
                            error!("could not request withdraw_from_game: {err}");
                        }
                        event.propagate(false);
                    })
                ),
                (
                    CurrentTurnRoot
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(5),
                        top: px(50),
                    }
                    Text::new(format!("current turn: {}", 0))
                )
            ]
        })
        .insert(DespawnOnExit(AppState::InGame));
}

fn spawn_bidding_ui(mut commands: Commands, conn: Res<StdbConn>, local_seat: Res<LocalSeat>) {
    let hands = conn.db().myhand();
    let hand_score = hands
        .iter()
        .find(|hand| hand.seat_id == local_seat.0)
        .map(|hand| format!("hand score: {}", hand.total_credit))
        .unwrap_or_default();

    commands
        .spawn_scene(bsn! {
            Node {
                position_type: PositionType::Absolute,
                bottom: px(0),
                left: px(0),
                right: px(0),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: px(10),
                column_gap: px(10),
            }
            Children [
                (
                    HandScoreText
                    Text::new(hand_score)
                    TextFont {
                        font_size: FontSize::Px(38.),
                    }
                ),
                (
                    LocalBidText
                    TextFont {
                        font_size: FontSize::Px(38.),
                    }
                ),
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        padding: px(10),
                        column_gap: px(10),
                    }
                    Children [
                        (
                            Node {
                                border: px(2),
                            }
                            BidTextInput
                            BorderColor::from(Color::from(SLATE_300))
                            EditableText {
                                visible_width: Option::Some(10.),
                                visible_lines: Option::Some(1.0),
                                allow_newlines: false,
                                max_characters: Option::Some(9),
                            }
                            TextFont {
                                font_size: FontSize::Px(38.),
                            }
                            EditableTextFilter::new(|c| c.is_digit(10))
                            TextLayout::no_wrap()
                            TextCursorStyle::default()
                            BackgroundColor(DARK_GREY)
                        ),
                        (
                            button("bid")
                            on(move |mut event: On<Pointer<Press>>, mut inputs: Query<&mut EditableText, With<BidTextInput>>, conn: Res<StdbConn>| {
                                for mut input in &mut inputs {
                                    if let Ok(bid_amount) = input.editor.text().to_string().parse::<i32>() {
                                        match conn.reducers().place_bid(bid_amount) {
                                            Ok(()) => input.clear(),
                                            Err(err) => error!("could not request place_bid: {err}"),
                                        }
                                    }
                                }
                                event.propagate(false);
                            })
                        ),
                        (
                            button("fold")
                            on(move |mut event: On<Pointer<Press>>, conn: Res<StdbConn>| {
                                if let Err(err) = conn.reducers().withdraw_from_bidding() {
                                    error!("could not request withdraw_from_bidding: {err}");
                                }
                                event.propagate(false);
                            })
                        ),
                    ]
                ),
                (
                    PlayerHandRoot
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        padding: px(10),
                        column_gap: px(10),
                    }
                )
            ]
        })
        .insert(DespawnOnExit(RoundState::Bidding));
}

fn spawn_showdown_ui(mut commands: Commands, round_winner: Res<RoundWinner>) {
    let round_winner = round_winner.0;
    commands
        .spawn_scene(bsn! {
            Node {
                position_type: PositionType::Absolute,
                right: px(0),
                left: px(0),
                top: px(0),
                bottom: px(0),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
            }
            Children [
                Text::new(format!("{} فاز بالشوط!", round_winner))
            ]
        })
        .insert(DespawnOnExit(RoundState::Showdown));
}

fn spawn_negotiation_ui(
    mut commands: Commands,
    current_game: Res<CurrentGame>,
    shaikh: Res<Shaikh>,
    local_seat: Res<LocalSeat>,
    conn: Res<StdbConn>,
    local_position: Res<LocalPosition>,
) {
    if local_seat.0 == shaikh.0 {
        commands
            .spawn_scene(bsn! {
                Node {
                    position_type: PositionType::Absolute,
                    bottom: px(0),
                    right: px(0),
                    left: px(0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                }
                Children [
                    (
                        text("أنت الشيخ 🫡")
                    ),
                    (
                        button("داقش")
                        on(|_event: On<Pointer<Press>>, conn: Res<StdbConn>| {
                            if let Err(err) = conn.reducers().start_showdown() {
                                error!("could not request start_showdown: {err}");
                            }
                        })
                    )
                ]
            })
            .insert(DespawnOnExit(RoundState::Negotiation));

        let seat_count = current_game.player_count.max(1);

        let seats = conn.db().seat();

        for seat in seats
            .iter()
            .filter(|seat| seat.game_id == current_game.id && seat.id != shaikh.0)
        {
            let player_position = seat.position;
            let relative =
                (local_position.0 + seat_count - player_position % seat_count) % seat_count;
            let (top, bottom, right, left) = match relative {
                1 => (percent(50.), auto(), auto(), px(0)),
                2 => (px(0), auto(), auto(), percent(50.)),
                3 => (percent(50.), auto(), px(0), auto()),
                _ => (auto(), auto(), auto(), auto()),
            };

            let player_id = seat.player_id;
            let seat_id = seat.id;

            commands
                .spawn_scene(bsn! {
                    NegotiationPrompt
                    PlayerMarker(player_id)
                    SeatId(seat_id)
                    PlayerPosition(player_position)
                    Node {
                        position_type: PositionType::Absolute,
                        top,
                        bottom,
                        right,
                        left,
                    }
                    Children [
                        (
                            NegotiationTextInput
                            SeatId(seat_id)
                            Node {
                                border: px(2),
                            }
                            BorderColor::from(Color::from(SLATE_300))
                            EditableText {
                                visible_width: Option::Some(10.),
                                visible_lines: Option::Some(1.0),
                                allow_newlines: false,
                                max_characters: Option::Some(9),
                            }
                            TextFont {
                                font_size: FontSize::Px(38.),
                            }
                            EditableTextFilter::new(|c| c.is_digit(10))
                            TextLayout::no_wrap()
                            TextCursorStyle::default()
                            BackgroundColor(DARK_GREY)
                        )
                        (
                            button("negotiate")
                            on(move |mut event: On<Pointer<Press>>, mut inputs: Query<(&mut EditableText, &SeatId), With<NegotiationTextInput>>, conn: Res<StdbConn>| {
                                if let Some((mut input, _)) = inputs.iter_mut().find(|(_, seat)| seat.0 == seat_id) {
                                    if let Ok(negotiated_amount) = input.editor.text().to_string().parse::<i32>() {
                                        match conn.reducers().negotiate_a_fold(seat_id, negotiated_amount) {
                                            Ok(()) => input.clear(),
                                            Err(err) => error!("could not request negotiate_a_fold: {err}"),
                                        }
                                    }
                                }
                                event.propagate(false);
                            })
                        ),
                    ]
                })
                .insert(DespawnOnExit(RoundState::Negotiation));
        }
    } else {
        let seat_count = current_game.player_count.max(1);

        let seats = conn.db().seat();

        for seat in seats.iter().filter(|seat| seat.game_id == current_game.id) {
            let player_position = seat.position;
            let relative =
                (local_position.0 + seat_count - player_position % seat_count) % seat_count;
            let (top, bottom, right, left) = match relative {
                1 => (percent(50.), auto(), auto(), px(0)),
                2 => (px(0), auto(), auto(), percent(50.)),
                3 => (percent(50.), auto(), px(0), auto()),
                _ => (auto(), auto(), auto(), auto()),
            };

            let player_id = seat.player_id;
            let seat_id = seat.id;

            if seat.id == shaikh.0 {
                commands.spawn_scene(bsn! {
                    PlayerMarker(player_id)
                    SeatId(seat_id)
                    PlayerPosition(player_position)
                    Node {
                        position_type: PositionType::Absolute,
                        top,
                        bottom,
                        right,
                        left,
                    }
                    Children [
                        (
                            text("الشيخ 🫡")
                        )
                    ]
                })
            } else {
                commands.spawn_scene(bsn! {
                    PlayerMarker(player_id)
                    SeatId(seat_id)
                    PlayerPosition(player_position)
                    Node {
                        position_type: PositionType::Absolute,
                        top,
                        bottom,
                        right,
                        left,
                    }
                    Children [
                    ]
                })
            }
            .insert(DespawnOnExit(RoundState::Negotiation));
        }
    }
}

fn new_negotiation(
    mut commands: Commands,
    shaikh: Res<Shaikh>,
    local_seat: Res<LocalSeat>,
    conn: Res<StdbConn>,
    mut negotiations: ReadInsertMessage<NegotiationOffer>,
    negotiation_prompts: Query<(Entity, &SeatId), With<NegotiationPrompt>>,
) {
    for msg in negotiations.read() {
        if msg.row.negotiator == local_seat.0 {
            if let Some((e, _)) = negotiation_prompts
                .iter()
                .find(|(_, seat_id)| seat_id.0 == msg.row.negotiated_with)
            {
                commands.entity(e).despawn();
            }
        } else if msg.row.negotiated_with == local_seat.0 {
            let negotiated_amount = msg.row.negotiated_amount;
            commands.spawn_scene(bsn! {
                ReceivedNegotiationOffer
                Node {
                    position_type: PositionType::Absolute,
                    right: px(0),
                    left: px(0),
                    top: px(0),
                    bottom: px(0),
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                }
                Children [
                    (Text::new(format!("الشيخ يعرض عليك {}", negotiated_amount))),
                    (
                        Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                        }
                        Children [
                            (
                                button("اقبل")
                                on(|_event: On<Pointer<Press>>, mut commands: Commands, conn: Res<StdbConn>| {
                                    if let Err(err) = conn.reducers().accept_negotiation() {
                                        error!("could not request accept_negotiation: {err}");
                                    }
                                })
                            ),
                            (
                                button("ارفض")
                                on(|_event: On<Pointer<Press>>, mut commands: Commands, conn: Res<StdbConn>| {
                                    if let Err(err) = conn.reducers().reject_negotiation() {
                                        error!("could not request reject_negotiation: {err}");
                                    }
                                })
                            ),
                        ]
                    )
                ]
            }).insert(DespawnOnExit(RoundState::Negotiation));
        }
    }
}

fn negotiation_rejected(
    mut commands: Commands,
    shaikh: Res<Shaikh>,
    local_seat: Res<LocalSeat>,
    conn: Res<StdbConn>,
    negotiation_offer: Single<Entity, With<ReceivedNegotiationOffer>>,
    mut rejected_negotiations: ReadInsertMessage<RejectedOffer>,
) {
    for msg in rejected_negotiations.read() {
        if msg.row.negotiated_with == local_seat.0 {
            let negotiated_amount = msg.row.negotiated_amount;
            commands.entity(*negotiation_offer).despawn();

            commands
                .spawn_scene(bsn! {
                    RejectedNegotiationOffer
                    Node {
                        position_type: PositionType::Absolute,
                        right: px(0),
                        left: px(0),
                        top: px(0),
                        bottom: px(0),
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                    }
                    Children [
                        (Text::new(format!("الشيخ عرض عليك {} ورفضت عرضه", negotiated_amount))),
                    ]
                })
                .insert(DespawnOnExit(RoundState::Negotiation));
        }
    }
}

fn negotiation_accepted(
    mut commands: Commands,
    shaikh: Res<Shaikh>,
    local_seat: Res<LocalSeat>,
    conn: Res<StdbConn>,
    negotiation_offer: Single<Entity, With<ReceivedNegotiationOffer>>,
    mut accepted_negotiations: ReadInsertMessage<AcceptedOffer>,
) {
    for msg in accepted_negotiations.read() {
        if shaikh.0 == local_seat.0 {}

        if msg.row.negotiated_with == local_seat.0 {
            let negotiated_amount = msg.row.negotiated_amount;
            commands.entity(*negotiation_offer).despawn();

            commands
                .spawn_scene(bsn! {
                    AcceptedNegotiationOffer
                    Node {
                        position_type: PositionType::Absolute,
                        right: px(0),
                        left: px(0),
                        top: px(0),
                        bottom: px(0),
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                    }
                    Children [
                        (Text::new(format!("الشيخ عرض عليك {} وقبلت عرضه", negotiated_amount))),
                    ]
                })
                .insert(DespawnOnExit(RoundState::Negotiation));
        }
    }
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
            .map(|c| {
                let card_name = format!("{:?} {:?}", c.rank, c.suit);

                bsn! {
                    Node {
                        border: px(2),
                    }
                    BorderColor::from(Color::BLACK)
                    Text::new(card_name)
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

#[cfg(not(feature = "debug"))]
fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

#[cfg(feature = "debug")]
fn spawn_camera(
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

fn main_menu_sub(subs: &mut StdbSubs, me: Identity) {
    subs.subscribe_query(SubKey::Player, |q| q.from.player());
    subs.subscribe_query(SubKey::Game, |q| q.from.game());
    subs.subscribe_query(SubKey::MySeat, move |q| {
        q.from.seat().r#where(|s| s.player_id.eq(me))
    });
}

fn subscribe_on_connect(
    mut commands: Commands,
    mut connected_msgs: ReadStdbConnectedMessage,
    mut subs: ResMut<StdbSubs>,
) {
    for msg in connected_msgs.read() {
        info!("connected as {:?}", msg.identity);
        commands.insert_resource(LocalPlayer(msg.identity));
        main_menu_sub(&mut subs, msg.identity);
    }
}

fn subscribe_to_game(subs: &mut StdbSubs, game_id: u64) {
    subs.subscribe_query(SubKey::Seat, move |q| {
        q.from.seat().r#where(|s| s.game_id.eq(game_id))
    });
    subs.subscribe_query(SubKey::PlayedCard, move |q| {
        q.from.played_card().r#where(|pc| pc.game_id.eq(game_id))
    });
    subs.subscribe_query(SubKey::PlacedBid, move |q| {
        q.from.placed_bid().r#where(|pb| pb.game_id.eq(game_id))
    });
    subs.subscribe_query(SubKey::NegotiationOffers, move |q| {
        q.from
            .negotiation_offer()
            .r#where(|n| n.game_id.eq(game_id))
    });
    subs.subscribe_query(SubKey::AcceptedOffers, move |q| {
        q.from.accepted_offer().r#where(|n| n.game_id.eq(game_id))
    });
    subs.subscribe_query(SubKey::RejectedOffers, move |q| {
        q.from.rejected_offer().r#where(|n| n.game_id.eq(game_id))
    });
    // XXX: is this needed?
    subs.subscribe_query(SubKey::PlayerHand, move |q| q.from.myhand());
    subs.subscribe_query(SubKey::Game, |q| {
        q.from.game().r#where(|g| g.id.eq(game_id))
    });
}

fn enter_lobby_when_seated(
    mut commands: Commands,
    mut subs: ResMut<StdbSubs>,
    conn: Res<StdbConn>,
    mut seats: ReadInsertMessage<Seat>,
    local: Res<LocalPlayer>,
) {
    for msg in seats.read() {
        if msg.row.player_id != local.0 {
            continue;
        }

        subscribe_to_game(&mut subs, msg.row.game_id);
        let game = conn
            .db()
            .game()
            .id()
            .find(&msg.row.game_id)
            .expect("game should exist");

        commands.insert_resource(CurrentGame {
            id: msg.row.game_id,
            player_count: game.player_count,
        });
        commands.insert_resource(LocalSeat(msg.row.id));
        commands.insert_resource(LocalPosition(msg.row.position));
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
        main_menu_sub(&mut subs, local.0);

        commands.remove_resource::<CurrentGame>();
        commands.remove_resource::<LocalSeat>();
        commands.set_state(AppState::MainMenu);
    }
}

fn handle_new_round(
    mut commands: Commands,
    current_game: Res<CurrentGame>,
    mut games: ReadUpdateMessage<Game>,
    online_bids: Query<Entity, With<OnlineBidText>>,
    local_bid: Single<Entity, With<LocalBidText>>,
) {
    for msg in games.read() {
        if msg.new.id != current_game.id {
            continue;
        }

        if msg.old.round != msg.new.round {
            commands.entity(*local_bid).insert(Text::default());
            for online_bid in online_bids {
                commands.entity(online_bid).insert(Text::default());
            }
        }
    }
}

fn handle_game_state(
    mut commands: Commands,
    mut current_game: ResMut<CurrentGame>,
    mut subs: ResMut<StdbSubs>,
    mut games: ReadInsertUpdateMessage<Game>,
) {
    for msg in games.read() {
        if msg.new.id != current_game.id {
            continue;
        }

        let new_game_state = msg
            .old
            .as_ref()
            .is_none_or(|old| old.state != msg.new.state)
            .then_some(msg.new.state.clone());

        if let Some(new_game_state) = new_game_state {
            match new_game_state {
                module_bindings::GameState::Lobby => {
                    subscribe_to_game(&mut subs, msg.new.id);
                    current_game.player_count = msg.new.player_count;

                    commands.set_state(AppState::InLobby);
                }
                module_bindings::GameState::Playing => {
                    subscribe_to_game(&mut subs, msg.new.id);
                    current_game.player_count = msg.new.player_count;

                    commands.set_state(AppState::InGame);
                }
                module_bindings::GameState::Ended(winner) => {
                    subs.unsubscribe(&SubKey::Seat).ok();
                    subs.unsubscribe(&SubKey::PlayedCard).ok();
                    // XXX: is this needed?
                    subs.unsubscribe(&SubKey::PlayerHand).ok();
                    subs.unsubscribe(&SubKey::Game).ok();

                    commands.remove_resource::<CurrentGame>();
                    commands.set_state(AppState::MainMenu);
                }
            }
        }

        let new_round_phase = msg
            .old
            .as_ref()
            .is_none_or(|old| old.round_phase != msg.new.round_phase)
            .then_some(msg.new.round_phase.clone());

        if let Some(new_round_phase) = new_round_phase {
            match new_round_phase {
                module_bindings::RoundPhase::Dealing => {}
                module_bindings::RoundPhase::Bidding => {
                    commands.remove_resource::<Shaikh>();
                    commands.remove_resource::<RoundWinner>();
                    commands.set_state(RoundState::Bidding);
                }
                module_bindings::RoundPhase::Negotiation(shaikh_seat_id) => {
                    // TODO: remove bidding resources
                    commands.set_state(RoundState::Negotiation);
                    commands.insert_resource(Shaikh(shaikh_seat_id));
                }
                module_bindings::RoundPhase::Showdown(winner_seat_id) => {
                    commands.remove_resource::<Shaikh>();
                    commands.set_state(RoundState::Showdown);
                    commands.insert_resource(RoundWinner(winner_seat_id));
                }
            }
        }
    }
}

fn handle_placed_bids(
    mut commands: Commands,
    mut bids: ReadInsertUpdateMessage<PlacedBid>,
    current_game: Res<CurrentGame>,
    local_seat_id: Res<LocalSeat>,
    bid_text: Single<Entity, With<LocalBidText>>,
    online_bid_texts: Query<(Entity, &PlayerPosition, &SeatId), With<OnlineBidText>>,
) {
    for msg in bids.read() {
        if msg.new.game_id != current_game.id {
            continue;
        }
        let new_bid = msg
            .old
            .as_ref()
            .is_none_or(|old| old.bidding_amount != msg.new.bidding_amount)
            .then_some(msg.new.bidding_amount);

        if let Some(new_bid) = new_bid {
            if msg.new.seat_id == local_seat_id.0 {
                commands
                    .entity(*bid_text)
                    .insert(Text::new(format!("Your bidding: {}", new_bid)));
            } else if let Some((entity, position, _)) = online_bid_texts
                .iter()
                .find(|(_, _, seat)| seat.0 == msg.new.seat_id)
            {
                commands
                    .entity(entity)
                    .insert(Text::new(format!("{}'s bidding: {}", position.0, new_bid)));
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

fn update_hand_score_text(
    mut player_hand: ReadInsertMessage<PlayerHand>,
    local_seat: Res<LocalSeat>,
    mut commands: Commands,
    hand_score_text: Single<Entity, With<HandScoreText>>,
) {
    for msg in player_hand.read() {
        if msg.row.seat_id != local_seat.0 {
            continue;
        }

        commands
            .entity(*hand_score_text)
            .insert(Text::new(format!("hand score: {}", msg.row.total_credit)));
    }
}

fn spawn_players_biddings(
    mut commands: Commands,
    conn: Res<StdbConn>,
    current_game: Res<CurrentGame>,
    local_position: Res<LocalPosition>,
) {
    let seat_count = current_game.player_count.max(1);

    let seats = conn.db().seat();

    for seat in seats.iter().filter(|seat| seat.game_id == current_game.id) {
        let player_position = seat.position;
        let relative = (local_position.0 + seat_count - player_position % seat_count) % seat_count;
        let (top, bottom, right, left) = match relative {
            1 => (percent(50.), auto(), auto(), px(0)),
            2 => (px(0), auto(), auto(), percent(50.)),
            3 => (percent(50.), auto(), px(0), auto()),
            _ => (auto(), auto(), auto(), auto()),
        };

        let player_id = seat.player_id;
        let seat_id = seat.id;

        commands
            .spawn_scene(bsn! {
                PlayerMarker(player_id)
                SeatId(seat_id)
                PlayerPosition(player_position)
                Node {
                    position_type: PositionType::Absolute,
                    top,
                    bottom,
                    right,
                    left,
                }
                Children [
                    (
                        OnlineBidText
                        SeatId(seat_id)
                        PlayerPosition(player_position)
                    )
                ]
            })
            .insert(DespawnOnExit(RoundState::Bidding));
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

#[cfg(debug_assertions)]
fn dev_start_solo_hotkey(keys: Res<ButtonInput<KeyCode>>, conn: Res<StdbConn>) {
    use crate::module_bindings::dev_start_solo;

    if keys.just_pressed(KeyCode::F5)
        && let Err(err) = conn.reducers().dev_start_solo()
    {
        error!("could not request dev_start_solo: {err}");
    }
}

#[cfg(debug_assertions)]
fn dev_respawn_ui_hotkey(
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
