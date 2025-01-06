use std::cell::RefCell;
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
    // Check if the .grit directory exists, return before even calling the function if it doesn't
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
    let x: Option<String> = helpers::get_current_head();
    // Context switch: variable gets rebound
    {
        println!("Most recent commit:");
        match x {
            Some(ref x) => {
                print!("[{x}]");
                let x: String =
                    helpers::get_commit_message(x).expect("Could not read previous commit message");
                println!(" {x}")
            }
            _ => (),
        };
    } // Context switch: variable is back to hash of the current HEAD

    let x: Option<String> = helpers::get_tree_of_commit(x.as_ref());
    let x: HashMap<String, String> = helpers::get_tree(x.as_ref());

    // Shadowing happens inside loop, but doesn't overwrite HashMap from outside scope
    println!("\nFiles tracked by grit:");
    x.iter().for_each(|(x, _)| println!("\t{x}"));

    let x: Vec<String> = helpers::get_index();

    // Shadowing happens inside loop, but doesn't overwrite HashMap from outside scope
    println!("\nChanges to be committed:");
    x.iter().for_each(|x| println!("\t{x}"));
}

fn add(x: Option<&String>) {
    match x {
        Some(x) => {
            // Context switch: variable is rebound
            {
                // Context switch: variable is rebound again
                {
                    let x: (&String, String) = (
                        x,
                        match fs::read_to_string(x) {
                            Ok(x) => x,
                            _ => {
                                println!("{x} does not match any file.");
                                return;
                            }
                        },
                    );

                    // This one is actually kind of disgusting. We need to compare the hash of the
                    // added file's contents to the hash of the same file in the
                    // current working tree. We can't use map_or etc. because that jumps into a new
                    // context where we no longer have access to the full tuple.
                    // Instead, just make a massive tuple and perform the check afterwards.
                    // NOTE: We have to do the read from the HashMap in two parts. Otherwise, compiler
                    // freaks out over variable lifetimes, probably due to the excessive use of as_ref?
                    let x: (&String, String, HashMap<String, String>) = (
                        x.0,
                        x.1,
                        helpers::get_tree(
                            helpers::get_tree_of_commit(helpers::get_current_head().as_ref())
                                .as_ref(),
                        ),
                    );

                    let x: (&String, String, Option<&String>) = (x.0, x.1, x.2.get(x.0));

                    if x.2.is_some() && x.2.unwrap().to_string() == helpers::hash_string(&x.1) {
                        println!("No changes to add...");
                        return;
                    }

                    let _ = write!(
                        File::create(format!(".grit/{}", helpers::hash_string(&x.1)))
                            .expect("Could not save added file"),
                        "{}",
                        x.1
                    );
                }
            } // Context switch: variable is again back to the path of the file to be added

            // Turn variable into tuple of the current index contents and the path of the file to
            // be added; Use RefCell to wrap vector with an immutable variable
            let x: (RefCell<Vec<String>>, &String) = (RefCell::new(helpers::get_index()), x);

            if !x.0.borrow().contains(&x.1) {
                x.0.borrow_mut().push(x.1.to_string());
            }

            let x: String = x.0.borrow().join("\n");

            let _ = write!(
                File::create(format!(".grit/index")).expect("Could not save added file"),
                "{}",
                x
            );
        }
        _ => {
            help();
            return;
        }
    }
}

fn commit(x: Option<&String>) {
    // The variable gets bound (by the match statement) to the path where the message should be
    // written, along with the message (which we get as a parameter)
    let x: (Option<&String>, (String, String)) = (
        x,
        match x {
            Some(_) => {
                // Bind x to the hash of the current HEAD and the contents of the new commit object
                let x: (Option<String>, Option<String>) =
                    (helpers::get_current_head(), helpers::get_current_head());
                let x: (Option<String>, Option<String>) =
                    (helpers::get_tree_of_commit(x.0.as_ref()), x.1);
                let x: (Option<String>, Option<String>) = (helpers::create_new_tree(x.0), x.1);
                if x.0.is_none() {
                    println!("Nothing to commit...");
                    return;
                }
                let x: (String, Option<String>) = (x.0.unwrap(), x.1);

                // The message is written only when returning from this scope
                let x = if x.1.is_some() {
                    format!("tree\t{}\nparent\t{}\n\n", x.0, x.1.unwrap())
                } else {
                    format!("tree\t{}\n\n", x.0)
                };

                // Bind variable to a tuple of the hash of the new commit object and its contents
                let x: (String, String) = (helpers::hash_string(&x), x);

                let _ = write!(
                    File::create(format!(".grit/HEAD")).expect("Could not open commit file"),
                    "{}",
                    x.0
                );

                // Empty the index
                let _ = File::create(format!(".grit/index")).expect("Could not open index file");

                // Return the path to the commit object along with its contents (without the commit
                // message which we can't access in this scope)
                x
            }
            _ => {
                help();
                return;
            }
        },
    );

    // Finally write the commit object contents (adding the commit message that the variable
    // contains again in this scope)
    let _ = write!(
        File::create(format!(".grit/{}", x.1 .0)).expect("Could not open commit file"),
        "{}",
        format!("{}{}", x.1 .1, x.0.unwrap_or(&"".to_string()))
    );
}

fn fuckgoback() {
    let x: Option<String> = helpers::get_current_head();
    if x.is_none() {
        println!("Could not read current HEAD");
        return;
    }

    let x = helpers::get_parent_of_commit(x.as_ref());
    match x {
        Some(x) => {
            let _ = write!(
                File::create(format!(".grit/HEAD")).expect("Could not open HEAD file"),
                "{}",
                x
            );

            let x: Option<String> = helpers::get_tree_of_commit(Some(&x));
            let x: HashMap<String, String> = helpers::get_tree(x.as_ref());

            x.iter().for_each(|x| {
                // classic (filepath, hash) -> (filepath, file contents) redefinition, you love to
                // see it
                let x = (
                    x.0,
                    fs::read_to_string(format!(".grit/{}", x.1))
                        .expect(&format!("Could not open object {} from previous tree", x.1)),
                );

                let _ = write!(
                    File::create(x.0)
                        .expect(&format!("Could not open file {} from previous tree", x.1)),
                    "{}",
                    x.1
                );
            });
        }
        _ => {
            println!("No previous commit");
            return;
        }
    }
}

fn main() {
    let x: i32 = 42;

    let x: Vec<String> = env::args().collect();
    let x: (Option<&String>, Option<&String>) = (x.get(1), x.get(2));

    match x {
        (Some(x), None) if x.as_str() == "init" => initialize(),
        (Some(x), None) if x.as_str() == "status" => check_initialized(|| status()),
        _ if x.0.is_some() && x.0.unwrap().as_str() == "add" && x.1.is_some() => {
            check_initialized(|| add(x.1))
        }
        _ if x.0.is_some() && x.0.unwrap().as_str() == "commit" && x.1.is_some() => {
            check_initialized(|| commit(x.1))
        }
        _ if x.0.is_some() && x.0.unwrap().as_str() == "fuckgoback" && x.1.is_none() => {
            check_initialized(|| fuckgoback())
        }
        _ => check_initialized(|| help()),
    };
}
