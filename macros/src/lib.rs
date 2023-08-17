extern crate proc_macro;

use std::fs;

use proc_macro::TokenStream;

#[proc_macro]
pub fn vec_sources_fn(input: TokenStream) -> TokenStream {
    // Convert the input TokenStream to a String
    let dir_path = input.to_string().trim_matches('"').to_string();

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
            fn_pointers.push(format!("{}::get_news,", filename));
        }
    }

    let expanded_code = format!("vec![{}]", fn_pointers.join(" "));

    expanded_code.parse().unwrap()
}
