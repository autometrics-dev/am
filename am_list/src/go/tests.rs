//! These tests are mostly for the queries, to ensure that querying only
//! autometricized functions, or all functions, give the correct set of
//! [`FunctionInfo`] entries. It is up to the [`Impl`] structure for each
//! language to then merge the sets so that functions that get detected by both
//! queries have their information merged.

use crate::{Location, Position, Range};

use super::*;
use pretty_assertions::assert_eq;

const FILE_NAME: &str = "source.go";

#[test]
fn detect_simple() {
    let source = r#"
        package lambda

        //autometrics:inst
        func the_one() {
        	return nil
        }
        "#;

    let query = AmQuery::try_new().unwrap();
    let list = query.list_function_names(FILE_NAME, source).unwrap();
    let all_query = AllFunctionsQuery::try_new().unwrap();
    let all_list = all_query.list_function_names(FILE_NAME, source).unwrap();

    let the_one_location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position {
                line: 4,
                column: 13,
            },
            end: Position {
                line: 4,
                column: 20,
            },
        },
    };

    let the_one_instrumented = FunctionInfo {
        id: ("lambda", "the_one").into(),
        instrumentation: Some(the_one_location.clone()),
        definition: Some(the_one_location.clone()),
    };

    let the_one_all_functions = FunctionInfo {
        id: ("lambda", "the_one").into(),
        instrumentation: None,
        definition: Some(the_one_location),
    };

    assert_eq!(list.len(), 1);
    assert_eq!(list[0], the_one_instrumented);

    assert_eq!(all_list.len(), 1);
    assert_eq!(all_list[0], the_one_all_functions);
}

#[test]
fn detect_legacy() {
    let source = r#"
        package beta

        func not_the_one() {
        }

        //autometrics:doc
        func sandwiched_function() {
        	return nil
        }

        func not_that_one_either() {
        }
        "#;

    let query = AmQuery::try_new().unwrap();
    let list = query.list_function_names(FILE_NAME, source).unwrap();
    let all_query = AllFunctionsQuery::try_new().unwrap();
    let all_list = all_query.list_function_names(FILE_NAME, source).unwrap();

    let not_the_one_location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position {
                line: 3,
                column: 13,
            },
            end: Position {
                line: 3,
                column: 13 + "not_the_one".len(),
            },
        },
    };

    let sandwiched_function_location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position {
                line: 7,
                column: 13,
            },
            end: Position {
                line: 7,
                column: 13 + "sandwiched_function".len(),
            },
        },
    };

    let not_that_one_either_location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position {
                line: 11,
                column: 13,
            },
            end: Position {
                line: 11,
                column: 13 + "not_that_one_either".len(),
            },
        },
    };

    let sandwiched_instrumented = FunctionInfo {
        id: ("beta", "sandwiched_function").into(),
        instrumentation: Some(sandwiched_function_location.clone()),
        definition: Some(sandwiched_function_location.clone()),
    };
    let sandwiched_all = FunctionInfo {
        id: ("beta", "sandwiched_function").into(),
        instrumentation: None,
        definition: Some(sandwiched_function_location.clone()),
    };
    let not_the_one = FunctionInfo {
        id: ("beta", "not_the_one").into(),
        instrumentation: None,
        definition: Some(not_the_one_location),
    };
    let not_that_one = FunctionInfo {
        id: ("beta", "not_that_one_either").into(),
        instrumentation: None,
        definition: Some(not_that_one_either_location),
    };

    assert_eq!(list.len(), 1);
    assert_eq!(list[0], sandwiched_instrumented);

    assert_eq!(
        all_list.len(),
        3,
        "complete functions list should have 3 items, got {} instead: {all_list:?}",
        all_list.len()
    );
    assert!(all_list.contains(&sandwiched_all));
    assert!(all_list.contains(&not_the_one));
    assert!(all_list.contains(&not_that_one));
}
