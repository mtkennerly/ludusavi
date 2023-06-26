/// Deserialization of GOG Game info file "$GAME_DIR/goggame-$GAME_ID.info"
#[derive(serde::Deserialize)]
pub struct GogGameInfo {
    pub name: String,
    // ignore everything else
}
