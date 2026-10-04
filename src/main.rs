use std::{collections::LinkedList, env::{self, current_dir}, fs::{ReadDir, read_dir, read_to_string}, path::PathBuf};

use crate::patterns::{SequenceExpr, form_expression_tree, test_sequence};

mod patterns;

fn main() {
    let args: Vec<String> = env::args().collect();
    let args_usage_msg = "Usage: dgrep <search text regex> <file name expression>";
    let Some(regex) = args.get(1) else {
        println!("{args_usage_msg}");
        return;
    };
    let Some(file_str) = args.get(2) else {
        println!("{args_usage_msg}");
        return;
    };

    let root_path: PathBuf;
    if file_str.starts_with('.') {
        let Ok(mut full_path) = current_dir() else {
            println!("Could not deterine current working directory. Maybe try an absolute path?");
            return;
        };
        full_path.push(file_str);
        root_path = full_path;
    } else {
        root_path = PathBuf::from(file_str);
    }
    if !root_path.is_dir() {
        println!("Not a directory: {file_str}");
        return;
    }

    let sequence_result = form_expression_tree(&regex);
    if let Err(err_msg) = sequence_result {
        println!("{err_msg}");
        return;
    }
    let sequence = sequence_result.expect("just confirmed it's OK");

    let matching_file_strs = test_sequence_on_files(root_path, sequence);

    println!("\nMatching File Paths:");

    if matching_file_strs.is_empty() {
        println!("No matches");
        return;
    }
    for file_path_str in matching_file_strs {
        println!("{file_path_str}");
    }
}

fn test_sequence_on_files(root_path: PathBuf, sequence: SequenceExpr) -> LinkedList<String> {
    let mut path_names: LinkedList<String> = LinkedList::new();
    let mut directory_queue: LinkedList<PathBuf> = LinkedList::new();
    directory_queue.push_back(root_path);
    let mut cur_dir_path: PathBuf;
    let mut child_path: PathBuf;
    let mut files: ReadDir;
    while !directory_queue.is_empty() {
        cur_dir_path = directory_queue.pop_front().expect("queue empty right after check");
        files = read_dir(cur_dir_path).expect("Could not open dir");
        for entry_result in files {
            let Ok(entry) = entry_result else {continue};
            child_path = entry.path();
            // possible both is_file() and is_dir() return false
            if child_path.is_file() {
                let Ok(contents) = read_to_string(&child_path) else {continue};
                if test_sequence(contents.as_str(), &sequence) {
                    path_names.push_back(child_path.into_string()
                        .unwrap_or(String::from("<unknown_path>")));
                }
            } else if child_path.is_dir() {
                directory_queue.push_back(child_path);
            }
        }
    }
    return path_names;
}

// File Name Matching Strategies

trait FileNameMatchingStrategy {
    fn does_match(&self, file_name: &str) -> bool;
}

struct DirectFileMatchingStrategy {
    file_name: String
}

impl FileNameMatchingStrategy for DirectFileMatchingStrategy {
    fn does_match(&self, file_name: &str) -> bool {
        self.file_name == file_name
    }
}

struct StartsWithFileMatchingStrategy {
    file_name_start: String
}

impl FileNameMatchingStrategy for StartsWithFileMatchingStrategy {
    fn does_match(&self, file_name: &str) -> bool {
        // trying not to use existing rust regex functinolaity, so can't use
        // str.starts_with, which accepts a rust Pattern ref
        file_name.len() >= self.file_name_start.len() &&
            &file_name[..self.file_name_start.len()] == file_name
    }
}

struct EndsWithFileMatchingStrategy {
    file_name_end: String
}

impl FileNameMatchingStrategy for EndsWithFileMatchingStrategy {
    fn does_match(&self, file_name: &str) -> bool {
        // trying not to used existing rust regex functinolaity, so can't use
        // str.ends_with, which accepts a rust Pattern ref
        file_name.len() >= self.file_name_end.len() &&
            &file_name[file_name.len() - self.file_name_end.len()..] == file_name
    }
}

struct StartsWithAndEndsWithFileSyptStrategy {
    starts_with_strategy: StartsWithFileMatchingStrategy,
    ends_with_strategy: EndsWithFileMatchingStrategy
}

impl FileNameMatchingStrategy for StartsWithAndEndsWithFileSyptStrategy {
    fn does_match(&self, file_name: &str) -> bool {
        self.starts_with_strategy.does_match(file_name) &&
            self.ends_with_strategy.does_match(file_name)
    }
}
