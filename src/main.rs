#![feature(iter_next_chunk)]

use std::{
    collections::HashMap,
    ffi::OsString,
    fs::File,
    io::{Read, Write},
    path::Path,
};

// Player name, [(game name, item name, location name, from pool, used)]
type PlandoList = HashMap<String, Vec<(String, String, String, bool, bool)>>;
// Player name, [(game name, item name)]
type FillerList = HashMap<String, Vec<(String, String)>>;

fn main() {
    let mut plando_list: PlandoList = HashMap::new();
    let mut plando_list_str = String::new();
    File::open("better_plando_list.txt")
        .expect("better_plando_list not found")
        .read_to_string(&mut plando_list_str)
        .expect("Failed to read better_plando_list file");

    let mut iter = plando_list_str.lines().filter(|line| !line.is_empty());
    while let Ok([name, game, item, location, from_pool]) = iter.next_chunk() {
        if let Some(vec) = plando_list.get_mut(name) {
            vec.push((
                game.to_owned(),
                item.to_owned(),
                location.to_owned(),
                from_pool == "pool",
                false,
            ));
        } else {
            plando_list.insert(
                name.to_owned(),
                vec![(
                    game.to_owned(),
                    item.to_owned(),
                    location.to_owned(),
                    from_pool == "pool",
                    false,
                )],
            );
        };
    }

    let mut filler_list: FillerList = HashMap::new();
    let mut filler_list_str = String::new();
    File::open("filler_list.txt")
        .expect("filler_list not found")
        .read_to_string(&mut filler_list_str)
        .expect("Failed to read filler_list file");

    let mut iter = filler_list_str.lines();

    while let Ok([name, game]) = iter.next_chunk() {
        let mut filler = vec![];

        for line in iter.by_ref() {
            if line.is_empty() {
                break;
            }

            filler.push((game.to_owned(), line.to_owned()));
        }

        if let Some(vec) = filler_list.get_mut(name) {
            vec.extend(filler);
        } else {
            filler_list.insert(name.to_owned(), filler);
        };
    }

    if let Ok(dir) = Path::new("./yamls").read_dir() {
        for yaml in dir.flatten() {
            add_plando(&yaml.file_name(), &mut plando_list, &mut filler_list);
        }
    }

    let mut unplaced = vec![];
    for (player, entries) in plando_list {
        let mut items = vec![];
        let mut any_unplaced = false;

        for (game, item, location, from_pool, placed) in entries {
            items.push((game, item, location, from_pool));
            if !placed {
                any_unplaced = true;
            }
        }

        if any_unplaced {
            unplaced.push((player, items));
        }
    }

    for (player, plando_items) in unplaced {
        let mut out_file = File::create(Path::new("./unplaced").join(format!("{player}.yaml")))
            .expect("Failed to create out file for unplaced items");

        let mut games = vec![];
        for (game, _, _, _) in &plando_items {
            if !games.contains(&game) {
                games.push(game);
            }
        }
        if let Some(filler_items) = filler_list.get(&player) {
            for (game, _) in filler_items {
                if !games.contains(&game) {
                    games.push(game);
                }
            }
        }

        for game in games {
            let mut plando_entries = vec![];

            for (item_game, item, location, from_pool) in &plando_items {
                if item_game == game {
                    plando_entries.push(format!("{{\"item\": \"{item}\", \"location\": \"{location}\", \"world\": \"The Cauldron\", \"force\": true, \"from_pool\": {from_pool}}}"));
                }
            }

            if let Some(filler_items) = filler_list.get(&player) {
                if filler_items.iter().any(|(item_game, _)| item_game == game) {
                    let mut locations = vec![];
                    for (item_game, _, location, _) in &plando_items {
                        if item_game != game {
                            locations.push(location.clone());
                        }
                    }

                    let mut items = vec![];
                    for (item_game, item) in filler_items {
                        if item_game == game {
                            items.push(format!("\"{item}\": {}", locations.len()));
                        }
                    }

                    plando_entries.push(format!("{{\"items\": {{{}}}, \"locations\": [{}], \"world\": \"The Cauldron\", \"force\": true, \"from_pool\": true}}", items.join(", "), locations.join(", ")));
                }
            }

            writeln!(&mut out_file, "{game}:").expect("Failed to write game");
            writeln!(
                &mut out_file,
                "  plando_items: [{}]",
                plando_entries.join(", ")
            )
            .expect("Failed to write plando");
        }
    }
}

fn add_plando(yaml: &OsString, plando_list: &mut PlandoList, filler_list: &mut FillerList) {
    let mut yaml_str = String::new();
    File::open(Path::new("./yamls").join(yaml))
        .expect("Failed to open yaml file")
        .read_to_string(&mut yaml_str)
        .expect("Failed to read yaml file");

    let mut name = String::new();
    let has_plando = yaml_str.contains("plando_items:");

    for line in yaml_str.lines() {
        if let Some((maybe_name_entry, maybe_name)) = line.split_once(':') {
            if maybe_name_entry == "name" || maybe_name_entry == "\u{feff}name" {
                name = maybe_name.trim().to_owned();
            }
        }
    }

    if name.is_empty() {
        panic!("yaml {:?} has no name", yaml);
    }

    if has_plando {
        println!("yaml {:?} already contains plando", yaml);
        return;
    }

    let mut out_file =
        File::create(Path::new("./dist").join(yaml)).expect("Failed to create out file for yaml");

    let mut plando_items = plando_list.get_mut(&name);
    let filler_items = filler_list.get(&name);

    for line in yaml_str.lines() {
        writeln!(&mut out_file, "{}", line).expect("Failed to write line");

        if !line.starts_with(' ') {
            if let Some((mut maybe_game, _)) = line.split_once(':') {
                maybe_game = maybe_game.trim_end();
                let mut plando_entries = vec![];
                let mut write_plando = false;

                if let Some(plando_items) = plando_items.as_deref_mut() {
                    for (game, item, location, from_pool, placed) in plando_items.iter_mut() {
                        if game == maybe_game {
                            write_plando = true;
                            *placed = true;
                            plando_entries.push(format!("{{\"item\": \"{item}\", \"location\": \"{location}\", \"world\": \"The Cauldron\", \"force\": true, \"from_pool\": {from_pool}}}"));
                        }
                    }
                }

                if let Some(filler_items) = filler_items {
                    if let Some(plando_items) = &plando_items {
                        if filler_items.iter().any(|(game, _)| game == maybe_game) {
                            let mut locations = vec![];
                            for (game, _, location, _, _) in plando_items.iter() {
                                if game != maybe_game {
                                    write_plando = true;
                                    locations.push(location.clone());
                                }
                            }

                            let mut items = vec![];
                            for (game, item) in filler_items {
                                if game == maybe_game {
                                    items.push(format!("\"{item}\": {}", locations.len()));
                                }
                            }

                            plando_entries.push(format!("{{\"items\": {{{}}}, \"locations\": [{}], \"world\": \"The Cauldron\", \"force\": true, \"from_pool\": true}}", items.join(", "), locations.join(", ")));
                        }
                    }
                }

                if write_plando {
                    writeln!(
                        &mut out_file,
                        "  plando_items: [{}]",
                        plando_entries.join(", ")
                    )
                    .expect("Failed to write plando");
                }
            }
        }
    }
}
