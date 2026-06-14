use ludusavi::api::*;

fn main() {
    let mut ludusavi = Ludusavi::load().unwrap();

    let games = vec![std::env::args().skip(1).next().unwrap_or_else(|| "Celeste".to_string())];

    let backups = ludusavi
        .list_backups(parameters::ListBackups { games: games.clone() })
        .unwrap();
    dbg!(backups);

    let output = ludusavi
        .back_up(parameters::BackUp {
            games,
            finality: Finality::Preview,
            ..Default::default()
        })
        .unwrap();
    dbg!(output);
}
