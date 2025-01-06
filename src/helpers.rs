use sha1::{Digest, Sha1};
use std::fs;

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn hash_string(s: &String) -> String {
    Sha1::digest(s.as_bytes())
        .iter()
        .map(|&x| format!("{:x}", x))
        .collect()
}

pub fn get_index() -> Vec<String> {
    // Create the index file if it doesn't already exist
    let x = ".grit/index";
    if !Path::new(x).exists() {
        let _ = File::create(x);
    }

    // Generate vector of filenames (removing possible empty lines)
    let x: String = fs::read_to_string(x).expect("Could not read index file");
    let x: Vec<String> = x.split("\n").map(|x| x.to_string()).collect();
    x.into_iter().filter(|x| x.len() > 0).collect()
}

pub fn get_commit_message(x: &String) -> Option<String> {
    let x = fs::read_to_string(format!(".grit/{x}"))
        .expect(&format!("Could not get commit message for hash {x}"));

    x.split("\n").last().map(|x| x.trim().to_string())
}

pub fn get_current_head() -> Option<String> {
    match fs::read_to_string(".grit/HEAD") {
        Ok(x) if !x.trim().is_empty() => Some(x),
        _ => None,
    }
}

// TODO: Combine the below two files into one that returns formatted "commit information"
pub fn get_parent_of_commit(x: Option<&String>) -> Option<String> {
    // Highly maintainable piece of code
    match x {
        Some(ref x) => match fs::read_to_string(format!(".grit/{x}")) {
            Ok(x) => x
                .split("\n")
                .nth(1)
                .map(|x| x.split_whitespace().nth(1))
                .flatten()
                .map(|x| x.to_string()),
            _ => None,
        },
        _ => None,
    }
}

pub fn get_tree_of_commit(x: Option<&String>) -> Option<String> {
    match x {
        Some(ref x) => match fs::read_to_string(format!(".grit/{x}")) {
            Ok(x) => x
                .split("\n")
                .next()
                .map(|x| x.split_whitespace().nth(1))
                .flatten()
                .map(|x| x.to_string()),
            _ => None,
        },
        _ => None,
    }
}

// The tree is represented by a HashMap of format (filepath, hash).
// As such, only one hash can be active for any single filepath.
pub fn get_tree(x: Option<&String>) -> HashMap<String, String> {
    match x {
        Some(x) if !x.is_empty() => {
            let x: String = fs::read_to_string(format!(".grit/{x}"))
                .expect("Could not open previous parent file");

            x.lines()
                .filter_map(|x| {
                    Some((
                        x.split_whitespace().nth(2)?.to_string(),
                        x.split_whitespace().nth(1)?.to_string(),
                    ))
                })
                .collect()
        }
        _ => HashMap::new(),
    }
}

// This function takes the hash of the parent commit's tree, and
// supplements it with changes from the index.
pub fn create_new_tree(x: Option<String>) -> Option<String> {
    // Produce a tuple (parent_tree_hash, filepaths)
    let x: (Option<String>, Vec<String>) = (x, get_index());
    if x.1.is_empty() {
        return None;
    }

    // Get mapping from hashes to filepaths from previous tree.
    let x: (HashMap<String, String>, Vec<String>) = (get_tree(x.0.as_ref()), x.1);

    // This tuple is of format (old_tree, new_tree)
    let x: (HashMap<String, String>, HashMap<String, String>) = (
        x.0,
        x.1.iter()
            .map(|x| {
                let x = (
                    x,
                    fs::read_to_string(x).expect(&format!("Could not read file {x} from index")),
                );
                (x.0.to_string(), hash_string(&x.1))
            })
            .collect(),
    );

    // Combine the previous tree with the new data
    let x =
        x.0.into_iter()
            .chain(x.1.into_iter())
            .collect::<HashMap<String, String>>();

    // Create contents of new tree
    let x: String = x
        .iter()
        .map(|x: (&String, &String)| format!("blob\t{}\t{}", x.1, x.0))
        .collect::<Vec<String>>()
        .join("\n");

    let x: (String, String) = (x.clone(), hash_string(&x));

    let _ = write!(
        File::create(format!(".grit/{}", x.1)).expect("Could not create tree object"),
        "{}",
        x.0
    );

    Some(x.1)
}
