use bevy::{
    prelude::*,
    ui_widgets::{SliderRange, SliderValue, slider_self_update},
};
use bevy_stdb::prelude::*;
use spacetimedb_sdk::table::TableLike;

use crate::player::*;
use crate::state::*;
use crate::stdb::*;
use crate::ui::{button, text};
use crate::{
    module_bindings::*,
    ui::{slider, slider_label},
};

const NEGOTIATION_STEP: f32 = 1000.0;
const NEGOTIATION_PRECISION: i32 = -3;

#[derive(Component, Debug, Default, Clone)]
pub struct NegotiationPrompt;

#[derive(Component, Debug, Default, Clone)]
pub struct ReceivedNegotiationOffer;

#[derive(Component, Debug, Default, Clone)]
pub struct RejectedNegotiationOffer;

#[derive(Component, Debug, Default, Clone)]
pub struct AcceptedNegotiationOffer;

#[derive(Component, Debug, Default, Clone)]
pub struct NegotiationSlider;

#[derive(Component, Debug, Default, Clone)]
pub struct NegotiationAmountText;

fn negotiation_bounds(conn: &StdbConn, seat_id: u64) -> (f32, f32) {
    let bid_amount = conn
        .db()
        .placed_bid()
        .iter()
        .find(|bid| bid.seat_id == seat_id)
        .map(|bid| bid.bidding_amount)
        .unwrap_or_default() as f32;

    let negotiations = conn
        .db()
        .negotiation_offer()
        .iter()
        .filter(|offer| offer.negotiator == seat_id)
        .map(|offer| offer.negotiated_amount as f32)
        .sum::<f32>();

    let accepted_offers = conn
        .db()
        .accepted_offer()
        .iter()
        .filter(|offer| offer.negotiator == seat_id)
        .map(|offer| offer.negotiated_amount as f32)
        .sum::<f32>();

    let max = ((bid_amount - negotiations - accepted_offers) / NEGOTIATION_STEP).floor()
        * NEGOTIATION_STEP;

    (0.0, max.max(0.0))
}

pub fn spawn_negotiation_ui(
    mut commands: Commands,
    current_game: Res<CurrentGame>,
    shaikh: Res<Shaikh>,
    local_seat: Res<LocalSeat>,
    conn: Res<StdbConn>,
    local_position: Res<LocalPosition>,
) {
    if local_seat.0 == shaikh.0 {
        let (min_negotiation, max_negotiation) = negotiation_bounds(&conn, local_seat.0);
        println!("{max_negotiation}");

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
                    @text("أنت الشيخ 🫡")
                    --
                    @button("داقش")
                    on(|_event: On<PointerPress>, conn: Res<StdbConn>| {
                        if let Err(err) = conn.reducers().start_showdown() {
                            error!("could not request start_showdown: {err}");
                        }
                    })
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
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        padding: px(10),
                        column_gap: px(10),
                        position_type: PositionType::Absolute,
                        top,
                        bottom,
                        right,
                        left,
                    }
                    Children [
                        NegotiationSlider
                        SeatId(seat_id)
                        @slider(min_negotiation, max_negotiation, min_negotiation, NEGOTIATION_STEP, NEGOTIATION_PRECISION)
                        on(slider_self_update)
                        on(slider_label::<NegotiationAmountText>)
                        --
                        NegotiationAmountText
                        @text(format!("{}", min_negotiation as i32))
                        TextFont {
                            font_size: FontSize::Px(38.),
                        }
                        --
                        @button("negotiate")
                        on(move |mut event: On<PointerPress>, sliders: Query<(&SliderValue, &SeatId), With<NegotiationSlider>>, conn: Res<StdbConn>| {
                            if let Some((value, _)) = sliders.iter().find(|(_, seat)| seat.0 == seat_id)
                                && let Err(err) = conn.reducers().negotiate_a_fold(seat_id, value.0.round() as i32)
                            {
                                error!("could not request negotiate_a_fold: {err}");
                            }
                            event.propagate(false);
                        })
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
                        @text("الشيخ 🫡")
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

pub fn update_negotiation_range(
    mut commands: Commands,
    conn: Res<StdbConn>,
    local_seat: Res<LocalSeat>,
    sliders: Query<(Entity, &SliderRange, &SliderValue), With<NegotiationSlider>>,
) {
    let (min, max) = negotiation_bounds(&conn, local_seat.0);
    let range = SliderRange::new(min, max);

    for (entity, current, value) in sliders.iter() {
        if *current == range {
            continue;
        }

        commands.entity(entity).try_insert(range);

        let clamped = range.clamp(value.0);
        if clamped != value.0 {
            commands.entity(entity).try_insert(SliderValue(clamped));
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
                    @text(format!("الشيخ يعرض عليك {}", negotiated_amount))
                    --
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                    }
                    Children [
                        @button("اقبل")
                        on(|_event: On<PointerPress>, _commands: Commands, conn: Res<StdbConn>| {
                            if let Err(err) = conn.reducers().accept_negotiation() {
                                error!("could not request accept_negotiation: {err}");
                            }
                        })
                        --
                        @button("ارفض")
                        on(|_event: On<PointerPress>, _commands: Commands, conn: Res<StdbConn>| {
                            if let Err(err) = conn.reducers().reject_negotiation() {
                                error!("could not request reject_negotiation: {err}");
                            }
                        })
                    ]
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
                        @text(format!("الشيخ عرض عليك {} ورفضت عرضه", negotiated_amount))
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
                        @text(format!("الشيخ عرض عليك {} وقبلت عرضه", negotiated_amount))
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
            (
                new_negotiation,
                negotiation_accepted,
                negotiation_rejected,
                update_negotiation_range,
            )
                .run_if(in_state(RoundState::Negotiation)),
        );
    }
}
