#[derive(Debug, Clone, PartialEq)]
pub enum GamePhase {
    MainMenu,
    CharacterCustomize { mode: GameMode },
    MultiplayerLobby,
    Playing,
    Paused,
    Settings { return_to: Box<GamePhase> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameMode {
    Story,
    Multiplayer,
}

impl Default for GamePhase {
    fn default() -> Self { GamePhase::MainMenu }
}
