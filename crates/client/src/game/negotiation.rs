use bevy::{
    color::palettes::{css::DARK_GREY, tailwind::SLATE_300},
    prelude::*,
    text::{EditableText, EditableTextFilter, TextCursorStyle},
};
use bevy_stdb::prelude::*;
use spacetimedb_sdk::table::TableLike;

use crate::module_bindings::*;
use crate::player::*;
use crate::state::*;
use crate::stdb::*;
use crate::ui::{button, text};

#[derive(Component, Debug, Default, Clone)]
pub struct NegotiationTextInput;

#[derive(Component, Debug, Default, Clone)]
pub struct NegotiationPrompt;

#[derive(Component, Debug, Default, Clone)]
pub struct ReceivedNegotiationOffer;

#[derive(Component, Debug, Default, Clone)]
pub struct RejectedNegotiationOffer;

#[derive(Component, Debug, Default, Clone)]
pub struct AcceptedNegotiationOffer;

pub fn spawn_negotiation_ui(
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

pub fn new_negotiation(
    mut commands: Commands,
    _shaikh: Res<Shaikh>,
    local_seat: Res<LocalSeat>,
    _conn: Res<StdbConn>,
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
                    (text(format!("الشيخ يعرض عليك {}", negotiated_amount))),
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
                                on(|_event: On<Pointer<Press>>, _commands: Commands, conn: Res<StdbConn>| {
                                    if let Err(err) = conn.reducers().accept_negotiation() {
                                        error!("could not request accept_negotiation: {err}");
                                    }
                                })
                            ),
                            (
                                button("ارفض")
                                on(|_event: On<Pointer<Press>>, _commands: Commands, conn: Res<StdbConn>| {
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

pub fn negotiation_rejected(
    mut commands: Commands,
    _shaikh: Res<Shaikh>,
    local_seat: Res<LocalSeat>,
    _conn: Res<StdbConn>,
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
                        (text(format!("الشيخ عرض عليك {} ورفضت عرضه", negotiated_amount))),
                    ]
                })
                .insert(DespawnOnExit(RoundState::Negotiation));
        }
    }
}

pub fn negotiation_accepted(
    mut commands: Commands,
    shaikh: Res<Shaikh>,
    local_seat: Res<LocalSeat>,
    _conn: Res<StdbConn>,
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
                        (text(format!("الشيخ عرض عليك {} وقبلت عرضه", negotiated_amount))),
                    ]
                })
                .insert(DespawnOnExit(RoundState::Negotiation));
        }
    }
}

pub struct NegotiationPlugin;
impl Plugin for NegotiationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(RoundState::Negotiation), spawn_negotiation_ui);

        app.add_systems(
            Update,
            (new_negotiation, negotiation_accepted, negotiation_rejected)
                .run_if(in_state(RoundState::Negotiation)),
        );
    }
}
