//! These tests are mostly for the queries, to ensure that querying only
//! autometricized functions, or all functions, give the correct set of
//! [`FunctionInfo`] entries. It is up to the [`Impl`] structure for each
//! language to then merge the sets so that functions that get detected by both
//! queries have their information merged.

use crate::{Location, Position, Range};

use super::*;
use pretty_assertions::assert_eq;

const FILE_NAME: &str = "source.rs";
const MODULE_NAME: &str = "dummy_mod";

#[test]
fn detect_single() {
    let source = r#"
        #[autometrics]
        fn main() {}
        "#;

    let list = AmQuery::try_new()
        .unwrap()
        .list_function_names(FILE_NAME, MODULE_NAME.to_string(), source)
        .unwrap();

    let location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position {
                line: 2,
                column: 11,
            },
            end: Position {
                line: 2,
                column: 15,
            },
        },
    };

    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0],
        FunctionInfo {
            id: (MODULE_NAME, "main").into(),
            instrumentation: Some(location.clone()),
            definition: Some(location),
        }
    );
}

#[test]
fn detect_impl_block() {
    let source = r#"
        struct Foo{};

        #[autometrics]
        impl Foo {
            fn method_a() {}
        }
        "#;

    let list = AmQuery::try_new()
        .unwrap()
        .list_function_names(FILE_NAME, MODULE_NAME.to_string(), source)
        .unwrap();

    let location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position {
                line: 5,
                column: 15,
            },
            end: Position {
                line: 5,
                column: 15 + "method_a".len(),
            },
        },
    };

    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0],
        FunctionInfo {
            id: (MODULE_NAME, "Foo::method_a").into(),
            instrumentation: Some(location.clone()),
            definition: Some(location),
        }
    );
}

#[test]
fn detect_trait_impl_block() {
    let source = r#"
        struct Foo{};

        #[autometrics]
        impl A for Foo {
            fn m_a() {}
        }
        "#;

    let list = AmQuery::try_new()
        .unwrap()
        .list_function_names(FILE_NAME, MODULE_NAME.to_string(), source)
        .unwrap();

    let location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position {
                line: 5,
                column: 15,
            },
            end: Position {
                line: 5,
                column: 15 + "m_a".len(),
            },
        },
    };

    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0],
        FunctionInfo {
            id: (MODULE_NAME, "Foo::m_a").into(),
            instrumentation: Some(location.clone()),
            definition: Some(location),
        }
    );
}

#[test]
fn dodge_wrong_impl_block() {
    let source = r#"
        struct Foo{};

        struct Bar{};

        impl Bar {
            fn method_one() {}
        }
        #[autometrics]
        impl Foo {
            fn method_two() {}
        }
        impl Bar {
            fn method_three() {}
        }
        #[autometrics]
        impl Foo {
            fn method_four() {}
        }
        "#;

    let list = AmQuery::try_new()
        .unwrap()
        .list_function_names(FILE_NAME, MODULE_NAME.to_string(), source)
        .unwrap();
    let all = AllFunctionsQuery::try_new()
        .unwrap()
        .list_function_names(FILE_NAME, MODULE_NAME.to_string(), source)
        .unwrap();

    let method_one_location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position {
                line: 6,
                column: 15,
            },
            end: Position {
                line: 6,
                column: 15 + "method_one".len(),
            },
        },
    };

    let method_two_location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position {
                line: 10,
                column: 15,
            },
            end: Position {
                line: 10,
                column: 15 + "method_two".len(),
            },
        },
    };

    let method_three_location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position {
                line: 13,
                column: 15,
            },
            end: Position {
                line: 13,
                column: 15 + "method_three".len(),
            },
        },
    };

    let method_four_location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position {
                line: 17,
                column: 15,
            },
            end: Position {
                line: 17,
                column: 15 + "method_four".len(),
            },
        },
    };

    let method_one = FunctionInfo {
        id: (MODULE_NAME, "Bar::method_one").into(),
        instrumentation: None,
        definition: Some(method_one_location),
    };
    let method_two = FunctionInfo {
        id: (MODULE_NAME, "Foo::method_two").into(),
        instrumentation: None,
        definition: Some(method_two_location.clone()),
    };
    let method_two_instrumented = FunctionInfo {
        id: (MODULE_NAME, "Foo::method_two").into(),
        instrumentation: Some(method_two_location.clone()),
        definition: Some(method_two_location),
    };
    let method_three = FunctionInfo {
        id: (MODULE_NAME, "Bar::method_three").into(),
        instrumentation: None,
        definition: Some(method_three_location),
    };
    let method_four = FunctionInfo {
        id: (MODULE_NAME, "Foo::method_four").into(),
        instrumentation: None,
        definition: Some(method_four_location.clone()),
    };
    let method_four_instrumented = FunctionInfo {
        id: (MODULE_NAME, "Foo::method_four").into(),
        instrumentation: Some(method_four_location.clone()),
        definition: Some(method_four_location),
    };

    assert_eq!(list.len(), 2);
    assert!(
        list.contains(&method_two_instrumented),
        "Expecting the list to contain {method_two_instrumented:?}\nComplete list is {list:?}"
    );
    assert!(
        list.contains(&method_four_instrumented),
        "Expecting the list to contain {method_four_instrumented:?}\nComplete list is {list:?}"
    );

    assert_eq!(all.len(), 4, "Complete list is {all:?}");
    assert!(
        all.contains(&method_one),
        "Expecting the list to contain {method_one:?}\nComplete list is {all:?}"
    );
    assert!(
        all.contains(&method_two),
        "Expecting the list to contain {method_two:?}\nComplete list is {all:?}"
    );
    assert!(
        all.contains(&method_three),
        "Expecting the list to contain {method_three:?}\nComplete list is {all:?}"
    );
    assert!(
        all.contains(&method_four),
        "Expecting the list to contain {method_four:?}\nComplete list is {all:?}"
    );
}

#[test]
fn detect_inner_module() {
    let source = r#"
        mod inner{
            #[autometrics]
            fn inner_function() {}
        }

        mod well{
            mod nested {
                mod stuff {
                    #[autometrics]
                    fn hidden_function() {}
                }
           }
        }
        "#;

    let list = AmQuery::try_new()
        .unwrap()
        .list_function_names(FILE_NAME, MODULE_NAME.to_string(), source)
        .unwrap();
    assert_eq!(
        list.len(),
        2,
        "Expected to find 2 items, instead the list is {list:?}"
    );

    let inner_fn_location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position {
                line: 3,
                column: 15,
            },
            end: Position {
                line: 3,
                column: 15 + "inner_function".len(),
            },
        },
    };
    let nested_fn_location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position {
                line: 10,
                column: 23,
            },
            end: Position {
                line: 10,
                column: 23 + "hidden_function".len(),
            },
        },
    };

    let inner_fn = FunctionInfo {
        id: (format!("{MODULE_NAME}::inner"), "inner_function").into(),
        instrumentation: Some(inner_fn_location.clone()),
        definition: Some(inner_fn_location.clone()),
    };
    assert!(
        list.contains(&inner_fn),
        "Expecting the detected functions to contain {inner_fn:?}\nComplete list is {list:?}"
    );
    let nested_fn = FunctionInfo {
        id: (
            format!("{MODULE_NAME}::well::nested::stuff"),
            "hidden_function",
        )
            .into(),
        instrumentation: Some(nested_fn_location.clone()),
        definition: Some(nested_fn_location),
    };
    assert!(
        list.contains(&nested_fn),
        "Expecting the detected functions to contain {nested_fn:?}\nComplete list is {list:?}"
    );
}

#[test]
fn detect_partially_annotated_impl_block() {
    let source = r#"
        struct Foo{};

        impl A for Foo {
            fn nothing_to_see_here() {}

            #[autometrics]
            fn m_a() {}
        }
        "#;

    let list = AmQuery::try_new()
        .unwrap()
        .list_function_names(FILE_NAME, MODULE_NAME.to_string(), source)
        .unwrap();
    let all = AllFunctionsQuery::try_new()
        .unwrap()
        .list_function_names(FILE_NAME, MODULE_NAME.to_string(), source)
        .unwrap();

    let dummy_location = Location {
        file: FILE_NAME.into(),
        range: Range {
            start: Position {
                line: 4,
                column: 15,
            },
            end: Position {
                line: 4,
                column: 15 + "nothing_to_see_here".len(),
            },
        },
    };
    let m_a_location = Location {
        file: FILE_NAME.into(),
        range: Range {
            start: Position {
                line: 7,
                column: 15,
            },
            end: Position {
                line: 7,
                column: 15 + "m_a".len(),
            },
        },
    };

    let m_a = FunctionInfo {
        id: (MODULE_NAME, "Foo::m_a").into(),
        instrumentation: None,
        definition: Some(m_a_location.clone()),
    };

    let m_a_instrumented = FunctionInfo {
        id: (MODULE_NAME, "Foo::m_a").into(),
        instrumentation: Some(m_a_location.clone()),
        definition: Some(m_a_location),
    };

    let dummy = FunctionInfo {
        id: (MODULE_NAME, "Foo::nothing_to_see_here").into(),
        instrumentation: None,
        definition: Some(dummy_location),
    };

    assert_eq!(list.len(), 1, "Complete list is {list:?}");
    assert!(
        list.contains(&m_a_instrumented),
        "Expecting the list to contain {m_a_instrumented:?}\nComplete list is {list:?}"
    );

    assert_eq!(all.len(), 2);
    assert!(
        all.contains(&m_a),
        "Expecting the list to contain {m_a:?}\nComplete list is {all:?}"
    );
    assert!(
        all.contains(&dummy),
        "Expecting the list to contain {dummy:?}\nComplete list is {all:?}"
    );
}
