use super::LaunchParser;

pub struct Legendary;
impl LaunchParser for Legendary {
    // TODO.2023-06-23 legendary .. launch LEGENDARY_GAME_ID
    // TODO.2023-06-23 legendary (EPIC) sample command:
    // Launch Command: /usr/bin/mangohud --dlsym /opt/Heroic/resources/app.asar.unpacked/build/bin/linux/legendary launch d8a4c98b5020483881eb7f0c3fc4cea3 --language en --wine /home/saschal/.config/heroic/tools/wine/Wine-GE-Proton7-31/bin/wine
    // read from /home/saschal/.config/legendary/metadata/d8a4c98b5020483881eb7f0c3fc4cea3.json
    // TODO.2023-06-23 handle other launchers (legendary, ...) here
    fn parse(&self, _commands: &[String]) -> Option<String> {
        println!("Legendary::parse: Legendary game detection not yet implemented.");
        None
    }
}
