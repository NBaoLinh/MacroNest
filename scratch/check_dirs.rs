use directories::ProjectDirs;

fn main() {
    if let Some(dirs) = ProjectDirs::from("com", "", "MacroNest") {
        println!("Data Local Dir: {:?}", dirs.data_local_dir());
    } else {
        println!("Could not find project dirs");
    }
}
