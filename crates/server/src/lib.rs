use std::collections::HashMap;

use spacetimedb::rand::seq::SliceRandom;
use spacetimedb::*;

const CARDS_PER_PLAYER: usize = 4;
const STARTING_BALANCE: i32 = 10_000;

#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Rank {
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

impl Rank {
    pub const ALL: [Rank; 13] = [
        Rank::Ace,
        Rank::Two,
        Rank::Three,
        Rank::Four,
        Rank::Five,
        Rank::Six,
        Rank::Seven,
        Rank::Eight,
        Rank::Nine,
        Rank::Ten,
        Rank::Jack,
        Rank::Queen,
        Rank::King,
    ];
}

#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

impl Suit {
    pub const ALL: [Suit; 4] = [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades];
}

#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Card {
    rank: Rank,
    suit: Suit,
}

impl Card {
    /// A fresh, unshuffled 52-card deck.
    pub fn full_deck() -> Vec<Card> {
        Suit::ALL
            .into_iter()
            .flat_map(|suit| Rank::ALL.into_iter().map(move |rank| Card { rank, suit }))
            .collect()
    }
}

// TODO: add `OfflinePlayer` or something instead of deleting the player
#[spacetimedb::table(accessor = player, public)]
pub struct Player {
    #[primary_key]
    pub identity: Identity,
}

#[derive(SpacetimeType, Debug, Clone, Copy)]
pub enum GameState {
    Lobby,
    Playing,
    Ended { winner: u64 },
}

#[derive(SpacetimeType, Debug, Clone, Copy)]
pub enum RoundPhase {
    Dealing,
    Bidding,
    Negotiation { shaikh: u64 },
    Showdown { winner: u64 },
}

#[spacetimedb::table(accessor = game, public)]
pub struct Game {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub state: GameState,
    pub round_phase: RoundPhase,
    pub round_start: u8,
    pub current_seat: u8,
    pub round: u8,
    pub plays_in_round: u8,
    pub player_count: u8,
}

#[derive(Debug)]
#[spacetimedb::table(accessor = seat, public, index(accessor = game_position, btree(columns = [game_id, position])))]
pub struct Seat {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[unique]
    pub player_id: Identity,
    #[index(btree)]
    pub game_id: u64,
    pub position: u8, // 0..game.player_count, turn order
    pub card_count: u32,
    pub balance: i32,
}

#[spacetimedb::table(accessor = player_hand)]
pub struct PlayerHand {
    #[primary_key]
    pub seat_id: u64,
    #[unique]
    pub player_id: Identity,
    #[index(btree)]
    pub game_id: u64,
    pub cards: Vec<Card>,
    pub total_credit: u64,
}

#[spacetimedb::table(accessor = deck)]
pub struct Deck {
    #[primary_key]
    pub game_id: u64,
    pub cards: Vec<Card>,
}

#[spacetimedb::table(accessor = placed_bid, public)]
pub struct PlacedBid {
    #[primary_key]
    pub seat_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub bidding_amount: i32,
}

#[spacetimedb::table(accessor = negotiation_offer, public)]
pub struct NegotiationOffer {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub negotiator: u64,
    #[unique]
    pub negotiated_with: u64,
    #[index(btree)]
    pub game_id: u64,
    pub negotiated_amount: i32,
}

#[spacetimedb::table(accessor = accepted_offer, public)]
pub struct AcceptedOffer {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub negotiator: u64,
    #[unique]
    pub negotiated_with: u64,
    #[index(btree)]
    pub game_id: u64,
    pub negotiated_amount: i32,
}

#[spacetimedb::table(accessor = rejected_offer, public)]
pub struct RejectedOffer {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub negotiator: u64,
    #[unique]
    pub negotiated_with: u64,
    #[index(btree)]
    pub game_id: u64,
    pub negotiated_amount: i32,
}

#[spacetimedb::table(accessor = played_card, public)]
pub struct PlayedCard {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub seat_id: u64,
    pub card: Card,
}

#[spacetimedb::reducer(init)]
pub fn init(ctx: &ReducerContext) -> Result<(), String> {
    log::debug!("Initializing...");
    Ok(())
}

#[spacetimedb::reducer(client_connected)]
pub fn identity_connected(ctx: &ReducerContext) {
    ctx.db
        .player()
        .try_insert(Player {
            identity: ctx.sender(),
        })
        .ok();
}

#[spacetimedb::reducer(client_disconnected)]
pub fn identity_disconnected(ctx: &ReducerContext) {
    // TODO: wait for timeout before deleting the player
    if let Some(leaving_seat) = ctx.db.seat().player_id().find(ctx.sender()) {
        if ctx.db.seat().game_id().filter(leaving_seat.game_id).count() == 1 {
            ctx.db.game().id().delete(leaving_seat.game_id);
        } else if let Some(game) = ctx.db.game().id().find(leaving_seat.game_id) {
            ctx.db.game().id().update(Game {
                player_count: game.player_count.saturating_sub(1),
                ..game
            });
        }
        #[cfg(feature = "dev")]
        {
            ctx.db.player_hand().game_id().delete(leaving_seat.game_id);
            ctx.db.placed_bid().game_id().delete(leaving_seat.game_id);
            ctx.db.seat().game_id().delete(leaving_seat.game_id);
            ctx.db.game().id().delete(leaving_seat.game_id);
        }
    }
    ctx.db.player_hand().player_id().delete(ctx.sender());
    ctx.db.seat().player_id().delete(ctx.sender());
    ctx.db.player().identity().delete(ctx.sender());
}

#[spacetimedb::reducer]
pub fn create_game(ctx: &ReducerContext) -> Result<(), String> {
    let Some(player) = ctx.db.player().identity().find(ctx.sender()) else {
        return Err(String::from(""));
    };
    let game = ctx.db.game().insert(Game {
        id: 0,
        state: GameState::Lobby,
        round_phase: RoundPhase::Dealing,
        round_start: 0,
        plays_in_round: 0,
        current_seat: 0,
        round: 0,
        player_count: 1,
    });

    ctx.db.seat().try_insert(Seat {
        id: 0,
        player_id: player.identity,
        game_id: game.id,
        card_count: 0,
        position: 0,
        balance: STARTING_BALANCE,
    })?;

    Ok(())
}

#[spacetimedb::reducer]
pub fn enter_game(ctx: &ReducerContext, game_id: u64) -> Result<(), String> {
    let Some(player) = ctx.db.player().identity().find(ctx.sender()) else {
        return Err(String::from("play not found"));
    };

    let Some(mut game) = ctx.db.game().id().find(game_id) else {
        return Err(String::from("game not found"));
    };

    let seats = ctx.db.seat().game_id().filter(game_id).count();
    match game.state {
        GameState::Lobby => {
            if seats < 4 {
                ctx.db.seat().try_insert(Seat {
                    id: 0,
                    player_id: player.identity,
                    game_id,
                    card_count: 0,
                    position: seats as u8,
                    balance: STARTING_BALANCE,
                })?;
                game.player_count += 1;
                ctx.db.game().id().update(game);
            } else {
                return Err(String::from("game is full"));
            }
        }
        GameState::Playing => return Err(String::from("game already started")),
        GameState::Ended { .. } => return Err(String::from("game ended")),
    }

    Ok(())
}

#[spacetimedb::reducer]
pub fn leave_game(ctx: &ReducerContext) -> Result<(), String> {
    let Some(leaving_seat) = ctx.db.seat().player_id().find(ctx.sender()) else {
        return Err(String::from("player is not in a game"));
    };
    let Some(mut game) = ctx.db.game().id().find(leaving_seat.game_id) else {
        return Err(String::from("game not found"));
    };
    if !matches!(game.state, GameState::Lobby) {
        return Err(String::from("cannot leave a game in progress"));
    }

    let seats: Vec<_> = ctx
        .db
        .seat()
        .game_position()
        .filter((game.id, (leaving_seat.position + 1)..))
        .collect();

    for seat in seats {
        ctx.db.seat().id().update(Seat {
            position: seat.position - 1,
            ..seat
        });
    }
    ctx.db.seat().id().delete(leaving_seat.id);

    if ctx.db.seat().game_id().filter(leaving_seat.game_id).count() == 0 {
        ctx.db.game().id().delete(leaving_seat.game_id);
    } else {
        game.player_count = game.player_count.saturating_sub(1);
        ctx.db.game().id().update(game);
    }

    Ok(())
}

#[spacetimedb::reducer]
pub fn withdraw_from_game(ctx: &ReducerContext) -> Result<(), String> {
    let Some(leaving_seat) = ctx.db.seat().player_id().find(ctx.sender()) else {
        return Err(String::from("player is not in a game"));
    };
    let Some(game) = ctx.db.game().id().find(leaving_seat.game_id) else {
        return Err(String::from("game not found"));
    };
    if !matches!(game.state, GameState::Playing) {
        return Err(String::from(
            "cannot withdraw if the game is not in progress",
        ));
    }

    // TODO: handle withdrawal properly here

    ctx.db.seat().game_id().delete(game.id);
    ctx.db.player_hand().game_id().delete(game.id);
    ctx.db.placed_bid().game_id().delete(game.id);
    ctx.db.game().id().delete(game.id);

    Ok(())
}

#[spacetimedb::reducer]
pub fn start_game(ctx: &ReducerContext, game_id: u64) -> Result<(), String> {
    let Some(mut game) = ctx.db.game().id().find(game_id) else {
        return Err(String::from("game not found"));
    };

    if !matches!(game.state, GameState::Lobby) {
        return Err(String::from("game already started"));
    }

    if ctx.db.seat().game_id().filter(game.id).count() != 4 {
        return Err(String::from("game needs 4 players to start"));
    }

    let mut cards = Card::full_deck();
    cards.shuffle(&mut ctx.rng());

    deal_hands(ctx, &game, &mut cards)?;

    // whatever is left after dealing is the draw pile
    ctx.db.deck().insert(Deck { game_id, cards });

    game.state = GameState::Playing;
    game.round_phase = RoundPhase::Bidding;
    ctx.db.game().id().update(game);

    Ok(())
}

#[spacetimedb::reducer]
pub fn play_card(ctx: &ReducerContext, idx: u32) -> Result<(), String> {
    let Some(seat) = ctx.db.seat().player_id().find(ctx.sender()) else {
        return Err(String::from("player doesn't have a seat"));
    };
    let Some(mut game) = ctx.db.game().id().find(seat.game_id) else {
        return Err(String::from("player is not in a game"));
    };

    if !matches!(game.state, GameState::Playing) {
        return Err(String::from("game is not in progress"));
    }

    if game.current_seat != seat.position {
        return Err(String::from("not your turn"));
    }

    let Some(mut player_hand) = ctx.db.player_hand().seat_id().find(seat.id) else {
        return Err(String::from("player doesn't have a playing hand"));
    };

    if idx as usize >= player_hand.cards.len() {
        return Err(String::from("no card with provided index"));
    }

    let card = player_hand.cards.remove(idx as usize);

    ctx.db.played_card().insert(PlayedCard {
        id: 0,
        game_id: seat.game_id,
        seat_id: seat.id,
        card,
    });

    ctx.db.player_hand().seat_id().update(player_hand);

    ctx.db.seat().id().update(Seat {
        card_count: seat.card_count - 1,
        ..seat
    });

    game.current_seat = (game.current_seat + 1) % 4;
    ctx.db.game().id().update(game);

    Ok(())
}

fn deal_hands(ctx: &ReducerContext, game: &Game, cards: &mut Vec<Card>) -> Result<(), String> {
    let mut seats: Vec<Seat> = ctx.db.seat().game_id().filter(game.id).collect();
    seats.sort_by_key(|seat| seat.position);

    if cards.len() < CARDS_PER_PLAYER * seats.len() {
        return Err(String::from("not enough cards left to deal"));
    }

    for seat in seats {
        let hand = cards.split_off(cards.len() - CARDS_PER_PLAYER);

        let total_credit = calculate_credit(&hand);

        ctx.db.player_hand().seat_id().delete(seat.id);
        ctx.db.player_hand().insert(PlayerHand {
            seat_id: seat.id,
            player_id: seat.player_id,
            game_id: game.id,
            cards: hand,
            total_credit,
        });
        ctx.db.seat().id().update(Seat {
            card_count: CARDS_PER_PLAYER as u32,
            ..seat
        });
    }

    Ok(())
}

fn calculate_credit(cards: &[Card]) -> u64 {
    let mut total_credit = 0;

    let mut sorted = cards.to_vec();
    sorted.sort_unstable();
    debug_assert!(sorted.len() == CARDS_PER_PLAYER);

    let mut duplicates: HashMap<Rank, Vec<Suit>> = HashMap::from([
        (Rank::Two, Vec::new()),
        (Rank::Three, Vec::new()),
        (Rank::Four, Vec::new()),
        (Rank::Five, Vec::new()),
        (Rank::Six, Vec::new()),
        (Rank::Seven, Vec::new()),
        (Rank::Eight, Vec::new()),
        (Rank::Nine, Vec::new()),
        (Rank::Ten, Vec::new()),
        (Rank::Jack, Vec::new()),
        (Rank::Queen, Vec::new()),
        (Rank::King, Vec::new()),
        (Rank::Ace, Vec::new()),
    ]);

    for card in cards {
        duplicates
            .entry(card.rank)
            .and_modify(|v| v.push(card.suit));
    }

    let is_series = sorted[0].rank as u64 + 1 == sorted[1].rank as u64
        && sorted[1].rank as u64 + 1 == sorted[2].rank as u64
        && sorted[2].rank as u64 + 1 == sorted[3].rank as u64;

    if let Some((r, _s)) = duplicates.iter().find(|(_r, s)| s.len() == 4) {
        total_credit += (*r as u64 + 1) * 10;
    } else if is_series {
        for card in sorted {
            total_credit += (card.rank as u64 + 1) * 4;
        }
    } else if let Some((r, _s)) = duplicates.iter().find(|(_r, s)| s.len() == 3) {
        total_credit += (*r as u64 + 1) * 3;
    } else if duplicates.iter().find(|(_r, s)| s.len() == 2).is_some() {
        let mut iter = duplicates.iter().filter(|(_r, s)| s.len() == 2);

        match (iter.next(), iter.next()) {
            (None, Some((r, _))) | (Some((r, _)), None) => {
                total_credit += (*r as u64 + 1) * 2;
            }
            (Some((r1, _)), Some((r2, _))) => {
                total_credit += (*r1 as u64 + 1) * 2;
                total_credit += (*r2 as u64 + 1) * 2;
            }
            (None, None) => {}
        }
    } else if let Some(max) = cards.iter().map(|c| c.rank as u64).max() {
        total_credit += max + 1;
    }

    total_credit
}

fn seat_positions(ctx: &ReducerContext, game_id: u64) -> Vec<u8> {
    let mut positions: Vec<u8> = ctx
        .db
        .seat()
        .game_id()
        .filter(game_id)
        .map(|seat| seat.position)
        .collect();
    positions.sort_unstable();
    positions
}

fn next_position(positions: &[u8], position: u8) -> u8 {
    positions
        .iter()
        .copied()
        .find(|&candidate| candidate > position)
        .or_else(|| positions.first().copied())
        .unwrap_or(position)
}

fn next_seat(ctx: &ReducerContext, mut game: Game) -> Result<(), String> {
    let positions = seat_positions(ctx, game.id);

    game.plays_in_round += 1;

    if game.plays_in_round as usize >= positions.len() {
        game.round += 1;
        game.round_start = next_position(&positions, game.round_start);
        game.current_seat = game.round_start;
        game.plays_in_round = 0;

        let bids: Vec<_> = ctx.db.placed_bid().game_id().filter(game.id).collect();

        match &bids[..] {
            // nobody wanted this hand, so it's dead: clear it and deal again
            [] => return begin_round(ctx, game),
            // no other bids, bidder wins immediately
            [winner_bid] => {
                settle_splits(ctx, &game, winner_bid.seat_id);
                game.round_phase = RoundPhase::Showdown {
                    winner: winner_bid.seat_id,
                };
            }
            _ => {
                if let Some(shaikh_bid) = bids.iter().max_by_key(|b| b.bidding_amount) {
                    game.round_phase = RoundPhase::Negotiation {
                        shaikh: shaikh_bid.seat_id,
                    };
                }
            }
        }
    } else {
        game.current_seat = next_position(&positions, game.current_seat);
    }

    ctx.db.game().id().update(game);

    Ok(())
}

#[spacetimedb::reducer]
pub fn place_bid(ctx: &ReducerContext, bidding_amount: i32) -> Result<(), String> {
    let Some(seat) = ctx.db.seat().player_id().find(ctx.sender()) else {
        return Err(String::from("player doesn't have a seat"));
    };
    let Some(game) = ctx.db.game().id().find(seat.game_id) else {
        return Err(String::from("player is not in a game"));
    };

    if !matches!(game.state, GameState::Playing) {
        return Err(String::from("game is not in progress"));
    }

    if !matches!(game.round_phase, RoundPhase::Bidding) {
        return Err(String::from("it's not bidding time"));
    }

    if game.current_seat != seat.position {
        return Err(String::from("not your turn"));
    }

    if seat.balance < bidding_amount {
        return Err(String::from("not enough balance"));
    }

    let max_bid = ctx
        .db
        .placed_bid()
        .game_id()
        .filter(game.id)
        .map(|pb| pb.bidding_amount)
        .max();

    if let Some(max_bid) = max_bid
        && bidding_amount <= max_bid
    {
        return Err(String::from("bid should be higher than previous bids"));
    }

    ctx.db.placed_bid().insert(PlacedBid {
        seat_id: seat.id,
        game_id: seat.game_id,
        bidding_amount,
    });

    next_seat(ctx, game)
}

#[spacetimedb::reducer]
pub fn withdraw_from_bidding(ctx: &ReducerContext) -> Result<(), String> {
    let Some(seat) = ctx.db.seat().player_id().find(ctx.sender()) else {
        return Err(String::from("player doesn't have a seat"));
    };
    let Some(game) = ctx.db.game().id().find(seat.game_id) else {
        return Err(String::from("player is not in a game"));
    };

    if !matches!(game.state, GameState::Playing) {
        return Err(String::from("game is not in progress"));
    }

    if game.current_seat != seat.position {
        return Err(String::from("not your turn"));
    }

    next_seat(ctx, game)
}

#[spacetimedb::reducer]
pub fn negotiate_a_fold(
    ctx: &ReducerContext,
    seat_negotiatied_with: u64,
    negotiated_amount: i32,
) -> Result<(), String> {
    let Some(seat) = ctx.db.seat().player_id().find(ctx.sender()) else {
        return Err(String::from("player doesn't have a seat"));
    };
    let Some(seat_negotiatied_with) = ctx.db.seat().id().find(seat_negotiatied_with) else {
        return Err(String::from("negotiation seat doesn't exist"));
    };
    let Some(game) = ctx
        .db
        .game()
        .id()
        .find(seat.game_id)
        .and(ctx.db.game().id().find(seat_negotiatied_with.game_id))
    else {
        return Err(String::from("player is not in a game"));
    };

    if !matches!(game.state, GameState::Playing) {
        return Err(String::from("game is not in progress"));
    }

    if let RoundPhase::Negotiation { shaikh: seat_id } = game.round_phase {
        if seat_id != seat.id {
            return Err(String::from("you're not the shaikh"));
        }
    } else {
        return Err(String::from("it's not negotiation time"));
    }

    let Some(original_bid) = ctx.db.placed_bid().seat_id().find(seat.id) else {
        return Err(String::from("no original bid"));
    };

    let mut negotiation_balance = original_bid.bidding_amount;

    for negotiation in ctx.db.negotiation_offer().negotiator().filter(seat.id) {
        negotiation_balance -= negotiation.negotiated_amount;
    }

    for negotiation in ctx.db.accepted_offer().negotiator().filter(seat.id) {
        negotiation_balance -= negotiation.negotiated_amount;
    }

    if negotiation_balance < negotiated_amount {
        return Err(String::from("negotiated amount is more than bid"));
    }

    ctx.db.negotiation_offer().insert(NegotiationOffer {
        id: 0,
        negotiator: seat.id,
        negotiated_with: seat_negotiatied_with.id,
        game_id: game.id,
        negotiated_amount,
    });

    Ok(())
}

#[spacetimedb::reducer]
pub fn accept_negotiation(ctx: &ReducerContext) -> Result<(), String> {
    let Some(seat) = ctx.db.seat().player_id().find(ctx.sender()) else {
        return Err(String::from("player doesn't have a seat"));
    };
    let Some(game) = ctx.db.game().id().find(seat.game_id) else {
        return Err(String::from("player is not in a game"));
    };

    if !matches!(game.state, GameState::Playing) {
        return Err(String::from("game is not in progress"));
    }

    if !matches!(game.round_phase, RoundPhase::Negotiation { .. }) {
        return Err(String::from("it's not negotiation time"));
    }

    let Some(negotiation_offer) = ctx.db.negotiation_offer().negotiated_with().find(seat.id) else {
        return Err(String::from(""));
    };

    ctx.db.accepted_offer().insert(AcceptedOffer {
        id: 0,
        negotiator: negotiation_offer.negotiator,
        negotiated_with: negotiation_offer.negotiated_with,
        game_id: negotiation_offer.game_id,
        negotiated_amount: negotiation_offer.negotiated_amount,
    });

    ctx.db.negotiation_offer().negotiated_with().delete(seat.id);

    Ok(())
}

#[spacetimedb::reducer]
pub fn reject_negotiation(ctx: &ReducerContext) -> Result<(), String> {
    let Some(seat) = ctx.db.seat().player_id().find(ctx.sender()) else {
        return Err(String::from("player doesn't have a seat"));
    };
    let Some(game) = ctx.db.game().id().find(seat.game_id) else {
        return Err(String::from("player is not in a game"));
    };

    if !matches!(game.state, GameState::Playing) {
        return Err(String::from("game is not in progress"));
    }

    if !matches!(game.round_phase, RoundPhase::Negotiation { .. }) {
        return Err(String::from("it's not negotiation time"));
    }

    let Some(negotiation_offer) = ctx.db.negotiation_offer().negotiated_with().find(seat.id) else {
        return Err(String::from(""));
    };

    ctx.db.rejected_offer().insert(RejectedOffer {
        id: 0,
        negotiator: negotiation_offer.negotiator,
        negotiated_with: negotiation_offer.negotiated_with,
        game_id: negotiation_offer.game_id,
        negotiated_amount: negotiation_offer.negotiated_amount,
    });

    ctx.db.negotiation_offer().negotiated_with().delete(seat.id);

    Ok(())
}

fn showdown_winner(ctx: &ReducerContext, game: &Game) -> Option<u64> {
    let folded: Vec<u64> = ctx
        .db
        .accepted_offer()
        .game_id()
        .filter(game.id)
        .map(|split| split.negotiated_with)
        .collect();

    let mut contenders: Vec<(u64, u64, u8)> = ctx
        .db
        .placed_bid()
        .game_id()
        .filter(game.id)
        .filter(|bid| !folded.contains(&bid.seat_id))
        .filter_map(|bid| {
            let hand = ctx.db.player_hand().seat_id().find(bid.seat_id)?;
            let seat = ctx.db.seat().id().find(bid.seat_id)?;
            Some((bid.seat_id, hand.total_credit, seat.position))
        })
        .collect();

    // XXX: on a tie the player with the lower position (sat in first) wins
    contenders.sort_by_key(|&(_, credit, position)| (std::cmp::Reverse(credit), position));

    contenders.first().map(|&(seat_id, _, _)| seat_id)
}

fn overall_winner(ctx: &ReducerContext, game: &Game) -> Option<u64> {
    let mut seats: Vec<Seat> = ctx.db.seat().game_id().filter(game.id).collect();
    // XXX: on a tie the player with the lower position (sat in first) wins
    seats.sort_by_key(|seat| (std::cmp::Reverse(seat.balance), seat.position));

    seats.first().map(|seat| seat.id)
}

fn settle_splits(ctx: &ReducerContext, game: &Game, winner: u64) {
    let mut pot = 0;

    for bid in ctx.db.placed_bid().game_id().filter(game.id) {
        pot += bid.bidding_amount;
        adjust_balance(ctx, bid.seat_id, -bid.bidding_amount);
    }

    for split in ctx.db.accepted_offer().game_id().filter(game.id) {
        pot -= split.negotiated_amount;
        adjust_balance(ctx, split.negotiated_with, split.negotiated_amount);
    }

    adjust_balance(ctx, winner, pot);
}

fn adjust_balance(ctx: &ReducerContext, seat_id: u64, delta: i32) {
    if let Some(seat) = ctx.db.seat().id().find(seat_id) {
        ctx.db.seat().id().update(Seat {
            balance: seat.balance + delta,
            ..seat
        });
    }
}

#[spacetimedb::reducer]
pub fn start_showdown(ctx: &ReducerContext) -> Result<(), String> {
    let Some(seat) = ctx.db.seat().player_id().find(ctx.sender()) else {
        return Err(String::from("player doesn't have a seat"));
    };
    let Some(mut game) = ctx.db.game().id().find(seat.game_id) else {
        return Err(String::from("player is not in a game"));
    };

    if !matches!(game.state, GameState::Playing) {
        return Err(String::from("game is not in progress"));
    }

    if !matches!(game.round_phase, RoundPhase::Negotiation { .. }) {
        return Err(String::from("it's not negotiation time"));
    }

    if ctx.db.negotiation_offer().game_id().filter(game.id).count() != 0 {
        return Err(String::from("all negotiations should be resolved first"));
    }

    let Some(winner) = showdown_winner(ctx, &game) else {
        return Err(String::from("nobody is left to win the round"));
    };

    settle_splits(ctx, &game, winner);

    // the round rests here so clients can reveal hands through `revealed_hands`;
    // the bids and splits stay on the table until `start_next_round` clears them
    game.round_phase = RoundPhase::Showdown { winner };
    ctx.db.game().id().update(game);

    Ok(())
}

#[spacetimedb::reducer]
pub fn start_next_round(ctx: &ReducerContext) -> Result<(), String> {
    let Some(seat) = ctx.db.seat().player_id().find(ctx.sender()) else {
        return Err(String::from("player doesn't have a seat"));
    };
    let Some(game) = ctx.db.game().id().find(seat.game_id) else {
        return Err(String::from("player is not in a game"));
    };

    if !matches!(game.state, GameState::Playing) {
        return Err(String::from("game is not in progress"));
    }

    if !matches!(game.round_phase, RoundPhase::Showdown { .. }) {
        return Err(String::from("the showdown isn't over"));
    }

    begin_round(ctx, game)
}

fn begin_round(ctx: &ReducerContext, mut game: Game) -> Result<(), String> {
    ctx.db.placed_bid().game_id().delete(game.id);
    ctx.db.accepted_offer().game_id().delete(game.id);
    ctx.db.rejected_offer().game_id().delete(game.id);
    ctx.db.played_card().game_id().delete(game.id);

    let Some(mut deck) = ctx.db.deck().game_id().find(game.id) else {
        return Err(String::from("game has no deck"));
    };

    let seats = ctx.db.seat().game_id().filter(game.id).count();
    if deck.cards.len() < CARDS_PER_PLAYER * seats {
        let Some(winner) = overall_winner(ctx, &game) else {
            return Err(String::from("game has no seats"));
        };

        game.state = GameState::Ended { winner };
        ctx.db.game().id().update(game);
        return Ok(());
    }

    deck.cards.shuffle(&mut ctx.rng());
    deal_hands(ctx, &game, &mut deck.cards)?;
    ctx.db.deck().game_id().update(deck);

    game.round_phase = RoundPhase::Bidding;
    ctx.db.game().id().update(game);

    Ok(())
}

#[spacetimedb::view(accessor = revealed_hands, public)]
pub fn revealed_hands(ctx: &ViewContext) -> Vec<PlayerHand> {
    let Some(seat) = ctx.db.seat().player_id().find(ctx.sender()) else {
        return Vec::new();
    };
    let Some(game) = ctx.db.game().id().find(seat.game_id) else {
        return Vec::new();
    };

    if !matches!(game.round_phase, RoundPhase::Showdown { .. }) {
        return Vec::new();
    }

    ctx.db.player_hand().game_id().filter(game.id).collect()
}

#[spacetimedb::view(accessor = myhand, public)]
pub fn myhand(ctx: &ViewContext) -> Option<PlayerHand> {
    ctx.db.player_hand().player_id().find(ctx.sender())
}

#[cfg(feature = "dev")]
#[spacetimedb::reducer]
pub fn dev_start_solo(ctx: &ReducerContext) -> Result<(), String> {
    if ctx.db.seat().player_id().find(ctx.sender()).is_some() {
        return Err(String::from("already seated"));
    }

    let game = ctx.db.game().insert(Game {
        id: 0,
        state: GameState::Lobby,
        round_phase: RoundPhase::Dealing,
        current_seat: 0,
        round_start: 0,
        plays_in_round: 0,
        round: 0,
        player_count: 4,
    });

    ctx.db.seat().try_insert(Seat {
        id: 0,
        player_id: ctx.sender(),
        game_id: game.id,
        position: 0,
        card_count: 0,
        balance: STARTING_BALANCE,
    })?;

    for position in 1..4 {
        ctx.db.seat().try_insert(Seat {
            id: 0,
            player_id: Identity::from_claims("dev", &format!("bot{position}")),
            game_id: game.id,
            position,
            card_count: 0,
            balance: STARTING_BALANCE,
        })?;
    }

    start_game(ctx, game.id)
}
