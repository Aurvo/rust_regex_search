use std::{collections::LinkedList, env::{self, current_dir}, fs::{File, ReadDir, read_dir}, io::{BufRead, BufReader}, path::PathBuf};

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

    let Ok(root_path) = current_dir() else {
        println!("Could not determine working directory.");
        return;
    };

    let sequence_result = form_expression_tree(&regex);
    if let Err(err_msg) = sequence_result {
        println!("{err_msg}");
        return;
    }
    let sequence = sequence_result.expect("just confirmed it's OK");

    let file_name_matcher_box;
    match determine_matcher_strategy(file_str.to_string()) {
        Ok(matcher_box) => file_name_matcher_box = matcher_box,
        Err(msg) => {
            println!("{msg}");
            return;
        }
    }

    test_sequence_on_files(root_path, sequence, &file_name_matcher_box.as_ref());
}

fn determine_matcher_strategy(matcher_str: String) -> Result<Box<dyn Fn(&str) -> bool>, String> {
    // Trying not to use existing rust regex utilities, which includes str.split().
    // Will need to implement split ourselves.    
    let mut ast_pose_opt: Option<usize> = None;
    for (byte_pose, ch) in matcher_str.char_indices() {
        if ch == '*' {
            match ast_pose_opt {
                Some(_) => return Err(String::from("Multiple *s in file name matcher text not supported.")),
                None => ast_pose_opt = Some(byte_pose)
            }
        }
    }

    let Some(ast_pose) = ast_pose_opt else {
        return Ok(Box::new(move |fname: &str| fname == matcher_str));
    };
    if matcher_str == "*" {
        return Ok(Box::new(|_| true))
    }
    // Note: size_of_val(&'*') is 4 like it is for any char.
    // We're not looking for htat size, here. We're looking for ths size a '*' character
    // takes up in a string slize.
    let size_of_ast = "*".len();
    let ending_str_start = ast_pose + size_of_ast;
    let starting_str = (&matcher_str)[..ast_pose].to_string();
    let ending_str = if ending_str_start < matcher_str.len() {
        (&matcher_str)[ending_str_start..].to_string()
    } else {
        String::new()
    };
    let does_str_match_start = move |fname: &str| {
        fname.len() >= starting_str.len() && fname[..starting_str.len()] == starting_str
    };
    let does_str_match_end = move |fname: &str| {
        fname.len() >= ending_str.len() && fname[fname.len() - ending_str.len()..] == ending_str
    };

    if ast_pose == 0 {
        return Ok(Box::new(does_str_match_end));
    } else if ast_pose == matcher_str.len() - size_of_ast {
        return Ok(Box::new(does_str_match_start));
    }
    return Ok(Box::new(move |fname: &str| {
        does_str_match_start(fname) && does_str_match_end(fname)
    }
    ));
}

fn test_sequence_on_files(root_path: PathBuf, sequence: SequenceExpr, does_file_name_match: &impl Fn(&str) -> bool) {
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
                print_matching_lines_in_file(child_path, &sequence, does_file_name_match);
            } else if child_path.is_dir() {
                directory_queue.push_back(child_path);
            }
        }
    }
}

fn print_matching_lines_in_file(file_path: PathBuf, sequence: &SequenceExpr, does_file_name_match: &impl Fn(&str) -> bool) {
    let Some(Some(file_name)) = file_path.file_name().map(|s| s.to_str()) else {return};
    if !does_file_name_match(file_name) {return}

    let Some(file_path_str) = file_path.to_str().map(|s| s.to_string()) else {return};
    let Ok(file) = File::open(file_path) else {return};
    let reader = BufReader::new(file);

    let mut printed_file_path = false;
    let mut line_num: u32 = 0;
    for line_result in reader.lines() {
        line_num += 1;
        let Ok(line) = line_result else {break};
        if !test_sequence(&line, sequence) {continue}
        if !printed_file_path {
            println!("{file_path_str}");
            println!("{:<10}{}", "LINE NUM", "LINE TEXT");
            printed_file_path = true;
        }
        println!("{line_num:<10}{line}")
    }
    if printed_file_path {println!()}
}