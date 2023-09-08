//! These tests are mostly for the queries, to ensure that querying only
//! autometricized functions, or all functions, give the correct set of
//! [`FunctionInfo`] entries. It is up to the [`Impl`] structure for each
//! language to then merge the sets so that functions that get detected by both
//! queries have their information merged.

use crate::{Location, Position, Range};

use super::*;
use pretty_assertions::assert_eq;

const DUMMY_MODULE: &str = "dummy";
const FILE_NAME: &str = "source.py";

#[test]
fn detect_simple() {
    let source = r#"
        from autometrics import autometrics

        @autometrics
        def the_one():
            return 'wake up, Neo'
        "#;

    let import_query = AmImportQuery::try_new().unwrap();
    let import_name = import_query.get_decorator_name(source).unwrap();
    let query = AmQuery::try_new(import_name.as_str()).unwrap();
    let list = query
        .list_function_names(FILE_NAME, source, DUMMY_MODULE)
        .unwrap();
    let all_query = AllFunctionsQuery::try_new().unwrap();
    let all_list = all_query
        .list_function_names(FILE_NAME, source, DUMMY_MODULE)
        .unwrap();

    let the_one_location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position {
                line: 4,
                column: 12,
            },
            end: Position {
                line: 4,
                column: 12 + "the_one".len(),
            },
        },
    };

    let the_one = FunctionInfo {
        id: ("dummy", "the_one").into(),
        instrumentation: None,
        definition: Some(the_one_location.clone()),
    };

    let the_one_instrumented = FunctionInfo {
        id: ("dummy", "the_one").into(),
        instrumentation: Some(the_one_location.clone()),
        definition: Some(the_one_location),
    };

    assert_eq!(list.len(), 1);
    assert_eq!(list[0], the_one_instrumented);
    assert_eq!(all_list.len(), 1);
    assert_eq!(all_list[0], the_one);
}

#[test]
fn detect_alias() {
    let source = r#"
        from autometrics import autometrics as am

        @am
        def the_one():
            return 'wake up, Neo'
        "#;

    let import_query = AmImportQuery::try_new().unwrap();
    let import_name = import_query.get_decorator_name(source).unwrap();
    let query = AmQuery::try_new(import_name.as_str()).unwrap();
    let list = query
        .list_function_names(FILE_NAME, source, DUMMY_MODULE)
        .unwrap();
    let all_query = AllFunctionsQuery::try_new().unwrap();
    let all_list = all_query
        .list_function_names(FILE_NAME, source, DUMMY_MODULE)
        .unwrap();

    let the_one_location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position {
                line: 4,
                column: 12,
            },
            end: Position {
                line: 4,
                column: 12 + "the_one".len(),
            },
        },
    };

    let the_one = FunctionInfo {
        id: ("dummy", "the_one").into(),
        instrumentation: None,
        definition: Some(the_one_location.clone()),
    };

    let the_one_instrumented = FunctionInfo {
        id: ("dummy", "the_one").into(),
        instrumentation: Some(the_one_location.clone()),
        definition: Some(the_one_location),
    };

    assert_eq!(list.len(), 1);
    assert_eq!(list[0], the_one_instrumented);
    assert_eq!(all_list.len(), 1);
    assert_eq!(all_list[0], the_one);
}

#[test]
fn detect_nested() {
    let source = r#"
        from autometrics import autometrics

        @autometrics
        def the_one():
            @autometrics
            def the_two():
                return 'wake up, Neo'
            return the_two()
        "#;

    let import_query = AmImportQuery::try_new().unwrap();
    let import_name = import_query.get_decorator_name(source).unwrap();
    let query = AmQuery::try_new(import_name.as_str()).unwrap();
    let list = query
        .list_function_names(FILE_NAME, source, DUMMY_MODULE)
        .unwrap();
    let all_query = AllFunctionsQuery::try_new().unwrap();
    let all_list = all_query
        .list_function_names(FILE_NAME, source, DUMMY_MODULE)
        .unwrap();

    let the_one_location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position {
                line: 4,
                column: 12,
            },
            end: Position {
                line: 4,
                column: 12 + "the_one".len(),
            },
        },
    };

    let the_two_location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position {
                line: 6,
                column: 16,
            },
            end: Position {
                line: 6,
                column: 16 + "the_two".len(),
            },
        },
    };

    let the_one = FunctionInfo {
        id: ("dummy", "the_one").into(),
        instrumentation: None,
        definition: Some(the_one_location.clone()),
    };
    let the_two = FunctionInfo {
        id: ("dummy", "the_one.<locals>.the_two").into(),
        instrumentation: None,
        definition: Some(the_two_location.clone()),
    };
    let the_one_instrumented = FunctionInfo {
        id: ("dummy", "the_one").into(),
        instrumentation: Some(the_one_location.clone()),
        definition: Some(the_one_location),
    };
    let the_two_instrumented = FunctionInfo {
        id: ("dummy", "the_one.<locals>.the_two").into(),
        instrumentation: Some(the_two_location.clone()),
        definition: Some(the_two_location),
    };

    assert_eq!(list.len(), 2);
    assert!(list.contains(&the_one_instrumented));
    assert!(list.contains(&the_two_instrumented));
    assert_eq!(all_list.len(), 2);
    assert!(all_list.contains(&the_one));
    assert!(all_list.contains(&the_two));
}
