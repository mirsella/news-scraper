use macros::vec_sources_fn;

mod test1;
mod test2;

#[test]
fn test_valid_sources() {
    let functions: Vec<fn(i32) -> i32> =
        vec_sources_fn!("macros/tests/vec_sources_fn", "get_function");

    // It should find 2 valid sources
    assert_eq!(functions.len(), 2);

    // Test if functions are correctly identified
    assert_eq!(functions[0](5), 6); // 5 + 1
    assert_eq!(functions[1](5), 10); // 5 * 2
}

// TODO: add tests for panicing macros
// #[test]
// #[should_panic(expected = "No such file or directory")]
// fn test_invalid_directory() {
//     // This should panic since the directory doesn't exist
//     let _functions: Vec<fn(i32) -> i32> = vec_sources_fn!("./tests/non_existent", "get_functions");
// }
//
// #[test]
// fn test_empty_directory() {
//     // If you had an empty directory, you'd expect the Vec to be empty.
//     let functions: Vec<fn(i32) -> i32> = vec_sources_fn!("./tests/empty", "get_functions");
//     assert_eq!(functions.len(), 0);
// }
