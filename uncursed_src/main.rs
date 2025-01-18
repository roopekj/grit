use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use std::env;
use std::fs;
use std::fs::DirBuilder;

mod helpers;

fn help() {
    println!("Usage:");
    println!("\tgrit init\t\t\tInitialize the current working directory as a grit repository.");
    println!("\tgrit status\t\t\tShow information about current tree.");
    println!("\tgrit add [FILEPATH]\t\tAdd file to index.");
    println!("\tgrit commit [COMMIT_MESSAGE]\tCommit changes from the index.");
    println!("\tgrit fuckgoback\t\t\tRevert to files from the previous commit and clear index.");
}

fn check_initialized<F>(function: F)
where
    F: FnOnce(),
{
    // Check if the .grit directory exists
    if !Path::new("./.grit").exists() {
        println!("Not a grit repository");
        return;
    }
    function();
}

fn initialize() {
    // Check that the .grit directory exists, create one if not
    let _ = DirBuilder::new().recursive(true).create(".grit");
}

fn status() {
    let current_head: Option<String> = helpers::get_current_head();
    println!("Most recent commit:");
    match current_head {
        Some(ref hash) => {
            let commit_message: String =
                helpers::get_commit_message(hash).expect("Could not read previous commit message");
            println!("[{hash}] {commit_message}")
        }
        _ => (),
    };

    let current_tree: Option<String> = helpers::get_tree_of_commit(current_head.as_ref());
    let current_tree: HashMap<String, String> = helpers::get_tree(current_tree.as_ref());

    println!("\nFiles tracked by grit:");
    current_tree
        .iter()
        .for_each(|(filepath, _)| println!("\t{filepath}"));

    let index: Vec<String> = helpers::get_index();
    println!("\nChanges to be committed:");
    index.iter().for_each(|filepath| println!("\t{filepath}"));
}

fn add(argument: Option<&String>) {
    match argument {
        Some(filepath) => {
            let contents = match fs::read_to_string(filepath) {
                Ok(contents) => contents,
                _ => {
                    println!("{filepath} does not match any file.");
                    return;
                }
            };
            let hash: String = helpers::hash_string(&contents);

            let current_commit: Option<String> = helpers::get_current_head();
            let current_tree: Option<String> = helpers::get_tree_of_commit(current_commit.as_ref());
            let current_tree: HashMap<String, String> = helpers::get_tree(current_tree.as_ref());
            if current_tree.get(filepath).map_or(false, |f| f == &hash) {
                println!("No changes to add...");
                return;
            }

            let mut file =
                File::create(format!(".grit/{hash}")).expect("Could not save added file");
            let _ = file.write_all(contents.as_bytes());

            let mut filepaths = helpers::get_index();

            if !filepaths.contains(&filepath) {
                filepaths.push(filepath.to_string());
            }
            let mut file = File::create(format!(".grit/index")).expect("Could not save added file");
            let _ = file.write_all(filepaths.join("\n").as_bytes());
        }
        _ => {
            help();
            return;
        }
    }
}

fn fuckgoback() {
    let current_head: Option<String> = helpers::get_current_head();
    if current_head.is_none() {
        println!("Could not read current HEAD");
        return;
    }

    let previous_commit = helpers::get_parent_of_commit(current_head.as_ref());
    match previous_commit {
        Some(previous_commit) => {
            let mut file = File::create(format!(".grit/HEAD")).expect("Could not open HEAD file");
            let _ = file.write_all(previous_commit.as_bytes());
            println!("{previous_commit}");

            let previous_tree: Option<String> = helpers::get_tree_of_commit(Some(&previous_commit));
            let previous_tree: HashMap<String, String> = helpers::get_tree(previous_tree.as_ref());

            previous_tree.iter().for_each(|(filepath, hash)| {
                let mut file = File::create(filepath).expect(&format!(
                    "Could not open file {filepath} from previous tree"
                ));
                let previous_contents = fs::read_to_string(format!(".grit/{hash}"))
                    .expect(&format!("Could not open object {hash} from previous tree"));

                let _ = file.write_all(previous_contents.as_bytes());
            });
        }
        _ => {
            println!("No previous commit");
            return;
        }
    }
}

fn commit(argument: Option<&String>) {
    match argument {
        Some(commit_message) => {
            let current_head: Option<String> = helpers::get_current_head();
            let current_tree_hash: Option<String> =
                helpers::get_tree_of_commit(current_head.as_ref());

            let current_tree_hash = helpers::create_new_tree(current_tree_hash);
            if current_tree_hash.is_none() {
                println!("Nothing to commit...");
                return;
            }
            let tree_hash = current_tree_hash.unwrap();

            let commit_content = match current_head {
                Some(ref parent_hash) => {
                    format!("tree\t{tree_hash}\nparent\t{parent_hash}\n\n{commit_message}")
                }
                _ => format!("tree\t{tree_hash}\n\n{commit_message}"),
            };

            let commit_hash = helpers::hash_string(&commit_content);
            let mut commit_file =
                File::create(format!(".grit/{commit_hash}")).expect("Could not open commit file");
            let _ = commit_file.write_all(commit_content.as_bytes());
            let mut head_file =
                File::create(format!(".grit/HEAD")).expect("Could not open commit file");
            let _ = head_file.write_all(commit_hash.as_bytes());

            // Empty the index
            let _ = File::create(format!(".grit/index")).expect("Could not open index file");
        }
        _ => {
            help();
            return;
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let command: Option<&String> = args.get(1);
    let argument: Option<&String> = args.get(2);

    match command {
        Some(c) if c.as_str() == "init" && argument.is_none() => initialize(),
        Some(c) if c.as_str() == "status" && argument.is_none() => check_initialized(|| status()),
        Some(c) if c.as_str() == "add" && argument.is_some() => check_initialized(|| add(argument)),
        Some(c) if c.as_str() == "commit" && argument.is_some() => {
            check_initialized(|| commit(argument))
        }
        Some(c) if c.as_str() == "fuckgoback" && argument.is_none() => {
            check_initialized(|| fuckgoback())
        }
        _ => check_initialized(|| help()),
    };
}
