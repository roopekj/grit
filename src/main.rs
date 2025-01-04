use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use sha1::{Digest, Sha1};
use std::env;
use std::fs;
use std::fs::DirBuilder;

fn help() {
    println!("Usage:");
    println!("\tgrit status\t\t\tShow information about current tree.");
    println!("\tgrit add [FILEPATH]\t\tAdd file to index.");
    println!("\tgrit commit [COMMIT_MESSAGE]\tCommit changes from the index.");
}

fn get_index() -> Vec<String> {
    let index_path = ".grit/index";
    if !Path::new(index_path).exists() {
        let _ = File::create(index_path);
    }
    let edited_files: String = fs::read_to_string(index_path).expect("Could not read index file");
    let mut filepaths: Vec<String> = edited_files.split("\n").map(|s| s.to_string()).collect();
    filepaths = filepaths.into_iter().filter(|a| a.len() > 0).collect();
    filepaths
}

fn hash_string(s: &String) -> String {
    let mut hasher = Sha1::new();
    hasher.update(s.as_bytes());
    let hex_string: String = hasher
        .finalize()
        .iter()
        .map(|&num| format!("{:x}", num))
        .collect();
    hex_string
}

fn get_tree(hash: Option<&String>) -> HashMap<String, String> {
    let mut tree: HashMap<String, String> = HashMap::new();
    match hash {
        Some(hash) if !hash.is_empty() => {
            let previous_tree: String = fs::read_to_string(format!(".grit/{hash}"))
                .expect("Could not open previous parent file");
            previous_tree.split("\n").for_each(|f| {
                if let (Some(hash), Some(fpath)) =
                    (f.split_whitespace().nth(1), f.split_whitespace().nth(2))
                {
                    tree.insert(fpath.to_string(), hash.to_string());
                }
            });
        }
        _ => (),
    };
    tree
}

fn get_commit_message(hash: &String) -> Option<String> {
    let contents = fs::read_to_string(format!(".grit/{hash}")).expect("a");

    contents
        .split("\n")
        .last()
        .map(|message| message.trim().to_string())
}

fn create_new_tree(parent_tree_hash: Option<String>) -> Option<String> {
    let filepaths: Vec<String> = get_index();
    if filepaths.is_empty() {
        // No files in index, do not create a new tree (or a commit)
        return None;
    }

    // Get mapping from hashes to filepaths from previous tree.
    let mut current_tree: HashMap<String, String> = get_tree(parent_tree_hash.as_ref());

    // Overwrite the previous tree's hashes with hashes from current file contents.
    filepaths.iter().for_each(|fpath| {
        let contents = fs::read_to_string(fpath).expect("Could not read file {fpath} from index");
        let hash = hash_string(&contents);
        let _ = current_tree.insert(fpath.to_string(), hash);
    });

    // Create contents of tree object
    let tree_content: String = current_tree
        .iter()
        .map(|(fpath, hash)| format!("blob\t{}\t{}", hash, fpath))
        .collect::<Vec<String>>()
        .join("\n");
    let tree_hash = hash_string(&tree_content);

    let mut file =
        File::create(format!(".grit/{tree_hash}")).expect("Could not create tree object");
    let _ = file.write_all(tree_content.as_bytes());

    Some(tree_hash)
}

fn status() {
    let parent_hash: Option<String> = get_parent_hash();
    println!("Most recent commit:");
    match parent_hash {
        Some(ref hash) => {
            let commit_message: String =
                get_commit_message(hash).expect("Could not read previous commit message");
            println!("[{hash}] {commit_message}")
        }
        _ => (),
    };

    let parent_tree_hash: Option<String> = get_parent_tree_hash(parent_hash.as_ref());
    let current_tree: HashMap<String, String> = get_tree(parent_tree_hash.as_ref());

    println!("\nFiles tracked by grit:");
    current_tree
        .iter()
        .for_each(|(filepath, _)| println!("\t{filepath}"));

    let index: Vec<String> = get_index();
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
            let hash: String = hash_string(&contents);

            let parent_hash: Option<String> = get_parent_hash();
            let parent_tree_hash: Option<String> = get_parent_tree_hash(parent_hash.as_ref());
            let current_tree: HashMap<String, String> = get_tree(parent_tree_hash.as_ref());
            if current_tree.get(filepath).map_or(false, |f| f == &hash) {
                println!("No changes to add...");
                return;
            }

            let mut file =
                File::create(format!(".grit/{hash}")).expect("Could not save added file");
            let _ = file.write_all(contents.as_bytes());

            let mut filepaths = get_index();

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

fn get_parent_hash() -> Option<String> {
    match fs::read_to_string(".grit/HEAD") {
        Ok(parent_hash) if !parent_hash.trim().is_empty() => Some(parent_hash),
        _ => None,
    }
}

fn get_parent_tree_hash(parent_hash: Option<&String>) -> Option<String> {
    match parent_hash {
        Some(ref parent_hash) => match fs::read_to_string(format!(".grit/{parent_hash}")) {
            Ok(parent_content) => parent_content
                .split("\n")
                .next()
                .map(|line| line.split_whitespace().nth(1))
                .flatten()
                .map(|s| s.to_string()),
            _ => None,
        },
        _ => None,
    }
}

fn commit(argument: Option<&String>) {
    match argument {
        Some(commit_message) => {
            let parent_hash: Option<String> = get_parent_hash();
            let parent_tree_hash: Option<String> = get_parent_tree_hash(parent_hash.as_ref());

            let parent_tree_hash = create_new_tree(parent_tree_hash);
            if parent_tree_hash.is_none() {
                println!("Nothing to commit...");
                return;
            }
            let tree_hash = parent_tree_hash.unwrap();

            let commit_content = match parent_hash {
                Some(ref parent_hash) => {
                    format!("tree\t{tree_hash}\nparent\t{parent_hash}\n\n{commit_message}")
                }
                _ => format!("tree\t{tree_hash}\n\n{commit_message}"),
            };

            let commit_hash = hash_string(&commit_content);
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
    // Check that the .grit directory exists, create one if not
    let _ = DirBuilder::new().recursive(true).create(".grit");

    // let yggdrasil: i32 = 42;
    let args: Vec<String> = env::args().collect();

    let command = args.get(1);
    let argument = args.get(2);

    match command {
        Some(c) if c.as_str() == "status" => status(),
        Some(c) if c.as_str() == "add" => add(argument),
        Some(c) if c.as_str() == "commit" => commit(argument),
        _ => help(),
    };
}
