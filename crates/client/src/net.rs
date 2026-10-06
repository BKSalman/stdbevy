use bevy::prelude::*;
use bevy_stdb::prelude::*;
use spacetimedb_sdk::Identity;

use crate::module_bindings::*;
use crate::stdb::*;
use crate::player::*;
use crate::state::*;

pub fn main_menu_sub(subs: &mut StdbSubs, me: Identity) {
    subs.subscribe_query(SubKey::Player, |q| q.from.player());
    subs.subscribe_query(SubKey::Game, |q| q.from.game());
    subs.subscribe_query(SubKey::MySeat, move |q| {
        q.from.seat().r#where(|s| s.player_id.eq(me))
    });
}

pub fn subscribe_on_connect(
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

pub fn subscribe_to_game(subs: &mut StdbSubs, game_id: u64) {
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

pub fn enter_lobby_when_seated(
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

pub fn exit_lobby_when_unseated(
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

pub fn handle_game_state(
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
                crate::module_bindings::GameState::Lobby => {
                    subscribe_to_game(&mut subs, msg.new.id);
                    current_game.player_count = msg.new.player_count;

                    commands.set_state(AppState::InLobby);
                }
                crate::module_bindings::GameState::Playing => {
                    subscribe_to_game(&mut subs, msg.new.id);
                    current_game.player_count = msg.new.player_count;

                    commands.set_state(AppState::InGame);
                }
                crate::module_bindings::GameState::Ended(_winner) => {
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
                crate::module_bindings::RoundPhase::Dealing => {}
                crate::module_bindings::RoundPhase::Bidding => {
                    commands.remove_resource::<Shaikh>();
                    commands.remove_resource::<RoundWinner>();
                    commands.set_state(RoundState::Bidding);
                }
                crate::module_bindings::RoundPhase::Negotiation(shaikh_seat_id) => {
                    // TODO: remove bidding resources
                    commands.set_state(RoundState::Negotiation);
                    commands.insert_resource(Shaikh(shaikh_seat_id));
                }
                crate::module_bindings::RoundPhase::Showdown(winner_seat_id) => {
                    commands.remove_resource::<Shaikh>();
                    commands.set_state(RoundState::Showdown);
                    commands.insert_resource(RoundWinner(winner_seat_id));
                }
            }
        }
    }
}

pub struct NetPlugin;
impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            subscribe_on_connect.run_if(resource_exists::<StdbConn>),
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
    }
}
