use sha1::{Digest, Sha1};
use std::fs;

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn hash_string(s: &String) -> String {
    let mut hasher = Sha1::new();
    hasher.update(s.as_bytes());
    let hex_string: String = hasher
        .finalize()
        .iter()
        .map(|&num| format!("{:x}", num))
        .collect();
    hex_string
}

pub fn get_index() -> Vec<String> {
    let index_path = ".grit/index";
    if !Path::new(index_path).exists() {
        let _ = File::create(index_path);
    }
    let edited_files: String = fs::read_to_string(index_path).expect("Could not read index file");
    let filepaths: Vec<String> = edited_files.split("\n").map(|s| s.to_string()).collect();
    filepaths.into_iter().filter(|a| a.len() > 0).collect()
}

pub fn get_commit_message(hash: &String) -> Option<String> {
    let contents = fs::read_to_string(format!(".grit/{hash}")).expect("a");

    contents
        .split("\n")
        .last()
        .map(|message| message.trim().to_string())
}

pub fn get_current_head() -> Option<String> {
    match fs::read_to_string(".grit/HEAD") {
        Ok(current_head) if !current_head.trim().is_empty() => Some(current_head),
        _ => None,
    }
}

pub fn get_parent_of_commit(commit_hash: Option<&String>) -> Option<String> {
    match commit_hash {
        Some(ref commit_hash) => match fs::read_to_string(format!(".grit/{commit_hash}")) {
            Ok(commit_content) => commit_content
                .split("\n")
                .nth(1)
                .map(|line| line.split_whitespace().nth(1))
                .flatten()
                .map(|s| s.to_string()),
            _ => None,
        },
        _ => None,
    }
}

pub fn get_tree_of_commit(parent_hash: Option<&String>) -> Option<String> {
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

pub fn get_tree(hash: Option<&String>) -> HashMap<String, String> {
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

pub fn create_new_tree(parent_tree_hash: Option<String>) -> Option<String> {
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
