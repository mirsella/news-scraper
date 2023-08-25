extern crate proc_macro;

use proc_macro::TokenStream;
use std::fs;

#[proc_macro]
pub fn vec_sources_fn(input: TokenStream) -> TokenStream {
    // Convert the input TokenStream to a String
    let input = input.to_string();

    // Extract directory and function name from the input
    let parts: Vec<&str> = input.split(',').collect();
    if parts.len() != 2 {
        panic!("Expected two arguments: directory path and function name");
    }

    let dir_path = parts[0]
        .trim_matches(|c| c == '"' || c == '(' || c == ' ')
        .to_string();
    let fn_name = parts[1]
        .trim_matches(|c| c == '"' || c == ')' || c == ' ')
        .to_string();

    let mut fn_pointers = vec![];
    for entry in fs::read_dir(&dir_path).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            let filename = entry.file_name().into_string().unwrap();
            if filename == "mod.rs"
                || filename == "lib.rs"
                || filename == "build.rs"
                || !filename.ends_with(".rs")
            {
                continue;
            }
            let filename = filename.trim_end_matches(".rs");
            if fn_name.is_empty() {
                fn_pointers.push(format!("{},", filename));
            } else {
                fn_pointers.push(format!("{}::{},", filename, fn_name));
            }
        }
    }

    let expanded_code = format!("vec![{}]", fn_pointers.join(" "));

    expanded_code.parse().unwrap()
}
