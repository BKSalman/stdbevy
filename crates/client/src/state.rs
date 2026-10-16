use bevy::prelude::*;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States, Reflect)]
pub enum AppState {
    #[default]
    MainMenu,
    InLobby,
    InGame,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, SubStates, Reflect)]
#[source(AppState = AppState::InGame)]
pub enum RoundState {
    #[default]
    Dealing,
    Bidding,
    Negotiation,
    Showdown,
}

#[cfg_attr(feature = "inspector", derive(Reflect))]
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct Shaikh(pub u64);

#[cfg_attr(feature = "inspector", derive(Reflect))]
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct RoundWinner(pub u64);

pub struct StatePlugin;
impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>().add_sub_state::<RoundState>();
    }
}
