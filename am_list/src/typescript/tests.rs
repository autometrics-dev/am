//! These tests are mostly for the queries, to ensure that querying only
//! autometricized functions, or all functions, give the correct set of
//! [`FunctionInfo`] entries. It is up to the [`Impl`] structure for each
//! language to then merge the sets so that functions that get detected by both
//! queries have their information merged.

use crate::{Location, Position, Range};

use super::{
    imports::{CanonicalSource, Identifier},
    queries::ImportsMapQuery,
    *,
};

use pretty_assertions::assert_eq;
use std::path::PathBuf;

const FILE_NAME: &str = "source.ts";
const MODULE_NAME: &str = "testingModule";

#[test]
fn detect_simple() {
    let source = r#"
import express from "express";
import { autometrics } from "@autometrics/autometrics";

const app = express();
const port = 8080;

function resolveAfterHalfSecond(): Promise<string> {
  return new Promise((resolve) => {
    setTimeout(() => {
      resolve("Function resolved");
    }, 500);
  });
}

const asyncCallMetricized = autometrics(async function asyncCall() {
  console.log("Calling async function");
  return await resolveAfterHalfSecond();
});
        "#;

    let list = AmQuery::try_new()
        .unwrap()
        .list_function_names(FILE_NAME, MODULE_NAME, source, None)
        .unwrap();
    let all = AllFunctionsQuery::try_new()
        .unwrap()
        .list_function_names(FILE_NAME, MODULE_NAME, source)
        .unwrap();
    let resolve_location = Location {
        file: FILE_NAME.into(),
        range: Range {
            start: Position { line: 7, column: 9 },
            end: Position {
                line: 7,
                column: 9 + "resolveAfterHalfSecond".len(),
            },
        },
    };
    let async_location = Location {
        file: FILE_NAME.into(),
        range: Range {
            start: Position {
                line: 15,
                column: 55,
            },
            end: Position {
                line: 15,
                column: 55 + "asyncCall".len(),
            },
        },
    };

    let resolve_after_half = FunctionInfo {
        id: (MODULE_NAME, "resolveAfterHalfSecond").into(),
        instrumentation: None,
        definition: Some(resolve_location),
    };
    let async_call = FunctionInfo {
        id: (MODULE_NAME, "asyncCall").into(),
        instrumentation: None,
        definition: Some(async_location.clone()),
    };
    let async_call_instrumented = FunctionInfo {
        id: (MODULE_NAME, "asyncCall").into(),
        instrumentation: Some(async_location),
        // TODO: async_call is instrumented using the wrapper function,
        // therefore AmQuery::list_function_names is not expected to guess that the definition is here as well
        //
        // But maybe it should, since the function is defined in place? It's low priority, because
        // AllFunctionsQuery is supposed to catch the definition and eventually we want to merge the
        // lists.
        definition: None,
    };

    assert_eq!(
        list.len(),
        1,
        "list should have 1 item, got this instead: {list:?}"
    );
    assert_eq!(list[0], async_call_instrumented);

    assert_eq!(
        all.len(),
        2,
        "list of all functions should have 2 items, got this instead: {all:?}"
    );
    assert!(
        all.contains(&async_call),
        "List of all functions should contain {async_call:?}; complete list is {all:?}"
    );
    assert!(
        all.contains(&resolve_after_half),
        "List of all functions should contain {resolve_after_half:?}; complete list is {all:?}"
    );
}

#[test]
fn detect_inner_route() {
    let source = r#"
import express from "express";
import { autometrics } from "@autometrics/autometrics";

const app = express();

app.get("/", rootRoute);
app.get("/bad", autometrics(badRoute));
app.get("/async", autometrics(asyncRoute));
        "#;

    let list = AmQuery::try_new()
        .unwrap()
        .list_function_names(FILE_NAME, MODULE_NAME, source, None)
        .unwrap();
    let all = AllFunctionsQuery::try_new()
        .unwrap()
        .list_function_names(FILE_NAME, MODULE_NAME, source)
        .unwrap();

    let bad_location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position {
                line: 7,
                column: 28,
            },
            end: Position {
                line: 7,
                column: 28 + "badRoute".len(),
            },
        },
    };
    let async_location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position {
                line: 8,
                column: 30,
            },
            end: Position {
                line: 8,
                column: 30 + "asyncRoute".len(),
            },
        },
    };

    let bad_route = FunctionInfo {
        id: (MODULE_NAME, "badRoute").into(),
        instrumentation: Some(bad_location),
        definition: None,
    };
    let async_route = FunctionInfo {
        id: (MODULE_NAME, "asyncRoute").into(),
        instrumentation: Some(async_location),
        definition: None,
    };

    assert_eq!(
        list.len(),
        2,
        "list should have 2 items, got this instead: {list:?}"
    );
    // In this example, as no function is _defined_ in the source code, we actually have
    // an empty list for "all functions"
    assert_eq!(
        all.len(),
        0,
        "list of all functions should have 2 items, got this instead: {all:?}"
    );

    assert!(
        list.contains(&bad_route),
        "The list should contain {bad_route:?}; complete list is {list:?}"
    );
    assert!(
        list.contains(&async_route),
        "The list should contain {async_route:?}; complete list is {list:?}"
    );
}

#[test]
fn detect_class() {
    let source = r#"
import express from "express";

@Autometrics
class Foo {
    x: number
    constructor(x = 0) {
        this.x = x;
    }
    method_b(): string {
        return "you win";
    }
}

class NotGood {
    x: string
    constructor(x = "got you") {
        this.x = x;
    }
    gotgot(): string {
        return "!";
    }
}
        "#;

    let list = AmQuery::try_new()
        .unwrap()
        .list_function_names(FILE_NAME, MODULE_NAME, source, None)
        .unwrap();
    let all = AllFunctionsQuery::try_new()
        .unwrap()
        .list_function_names(FILE_NAME, MODULE_NAME, source)
        .unwrap();

    let foo_constructor_location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position { line: 6, column: 4 },
            end: Position {
                line: 6,
                column: 4 + "constructor".len(),
            },
        },
    };
    let foo_method_b_location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position { line: 9, column: 4 },
            end: Position {
                line: 9,
                column: 4 + "method_b".len(),
            },
        },
    };
    let not_good_constructor_location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position {
                line: 16,
                column: 4,
            },
            end: Position {
                line: 16,
                column: 4 + "constructor".len(),
            },
        },
    };
    let not_good_gotgot_location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position {
                line: 19,
                column: 4,
            },
            end: Position {
                line: 19,
                column: 4 + "gotgot".len(),
            },
        },
    };

    let foo_constructor_instrumented = FunctionInfo {
        id: (MODULE_NAME, "Foo.constructor").into(),
        instrumentation: Some(foo_constructor_location.clone()),
        definition: Some(foo_constructor_location.clone()),
    };
    let method_b_instrumented = FunctionInfo {
        id: (MODULE_NAME, "Foo.method_b").into(),
        instrumentation: Some(foo_method_b_location.clone()),
        definition: Some(foo_method_b_location.clone()),
    };
    let foo_constructor = FunctionInfo {
        id: (MODULE_NAME, "Foo.constructor").into(),
        instrumentation: None,
        definition: Some(foo_constructor_location),
    };
    let method_b = FunctionInfo {
        id: (MODULE_NAME, "Foo.method_b").into(),
        instrumentation: None,
        definition: Some(foo_method_b_location),
    };
    let not_good_constructor = FunctionInfo {
        id: (MODULE_NAME, "NotGood.constructor").into(),
        instrumentation: None,
        definition: Some(not_good_constructor_location),
    };
    let gotgot_method = FunctionInfo {
        id: (MODULE_NAME, "NotGood.gotgot").into(),
        instrumentation: None,
        definition: Some(not_good_gotgot_location),
    };

    assert_eq!(
        list.len(),
        2,
        "list should have 2 items, got this instead: {list:?}"
    );
    assert_eq!(
        all.len(),
        4,
        "list of all functions should have 4 items, got this instead: {all:?}"
    );

    assert!(
        list.contains(&foo_constructor_instrumented),
        "The list should contain {foo_constructor_instrumented:?}; complete list is {list:?}"
    );
    assert!(
        list.contains(&method_b_instrumented),
        "The list should contain {method_b_instrumented:?}; complete list is {list:?}"
    );

    assert!(
        all.contains(&foo_constructor),
        "The list of all functions should contain {foo_constructor:?}; complete list is {all:?}"
    );
    assert!(
        all.contains(&method_b),
        "The list of all functions should contain {method_b:?}; complete list is {all:?}"
    );
    assert!(
        all.contains(&not_good_constructor),
        "The list of all functions should contain {not_good_constructor:?}; complete list is {all:?}"
    );
    assert!(
        all.contains(&gotgot_method),
        "The list of all functions should contain {gotgot_method:?}; complete list is {all:?}"
    );
}

#[test]
fn compute_import_map() {
    let source = r#"
import { exec } from 'child_process'
import { anyRoute as myRoute } from './handlers'
import * as other from '../other'
import { autometrics } from '@autometrics/autometrics'

const instrumentedExec = autometrics(exec);
const instrumentedRoute = autometrics(myRoute);
const instrumentedOther = autometrics(other.stuff);
        "#;

    let imports_query = ImportsMapQuery::try_new().expect("can build the imports map query");
    let imports_map = imports_query
        .list_imports(Some(&PathBuf::try_from("src/").unwrap()), source)
        .expect("can build the imports map from a query");

    let other_import = CanonicalSource::from("sibling://other");
    let exec_import = (
        Identifier::from("exec"),
        CanonicalSource::from("ext://child_process"),
    );
    let route_import = (
        Identifier::from("anyRoute"),
        CanonicalSource::from("src/handlers"),
    );
    let autometrics_import = (
        Identifier::from("autometrics"),
        CanonicalSource::from("ext://@autometrics/autometrics"),
    );

    assert_eq!(
        imports_map
            .find_namespace(&Identifier::from("other"))
            .unwrap(),
        other_import
    );
    assert_eq!(
        imports_map
            .find_identifier(&Identifier::from("exec"))
            .unwrap(),
        exec_import
    );
    assert_eq!(
        imports_map
            .find_identifier(&Identifier::from("myRoute"))
            .unwrap(),
        route_import
    );
    assert_eq!(
        imports_map
            .find_identifier(&Identifier::from("autometrics"))
            .unwrap(),
        autometrics_import
    );
}

#[test]
fn detect_imported_functions() {
    let source = r#"
import { exec } from 'child_process'
import { anyRoute as myRoute } from './handlers'
import * as other from '../other'
import { autometrics } from '@autometrics/autometrics'

const instrumentedExec = autometrics(exec);
const instrumentedRoute = autometrics(myRoute);
const instrumentedOther = autometrics(other.stuff);
        "#;

    let list = AmQuery::try_new()
        .unwrap()
        .list_function_names(FILE_NAME, MODULE_NAME, source, Some(&PathBuf::from("src/")))
        .unwrap();
    let all = AllFunctionsQuery::try_new()
        .unwrap()
        .list_function_names(FILE_NAME, MODULE_NAME, source)
        .unwrap();

    let exec_location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position {
                line: 6,
                column: 37,
            },
            end: Position {
                line: 6,
                column: 37 + "exec".len(),
            },
        },
    };
    let route_location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position {
                line: 7,
                column: 38,
            },
            end: Position {
                line: 7,
                column: 38 + "myRoute".len(),
            },
        },
    };
    let other_location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position {
                line: 8,
                column: 38,
            },
            end: Position {
                line: 8,
                column: 38 + "other.stuff".len(),
            },
        },
    };

    let exec = FunctionInfo {
        id: ("ext://child_process", "exec").into(),
        instrumentation: Some(exec_location),
        definition: None,
    };
    let any_route = FunctionInfo {
        id: ("src/handlers", "anyRoute").into(),
        instrumentation: Some(route_location),
        definition: None,
    };
    let stuff = FunctionInfo {
        id: ("sibling://other", "stuff").into(),
        instrumentation: Some(other_location),
        definition: None,
    };

    assert_eq!(
        list.len(),
        3,
        "list should have 3 items, got this instead: {list:?}"
    );
    assert!(
        list.contains(&exec),
        "List of instrumented functions should contain {exec:?}. Complete list is {list:?}"
    );
    assert!(
        list.contains(&any_route),
        "List of instrumented functions should contain {any_route:?}. Complete list is {list:?}"
    );
    assert!(
        list.contains(&stuff),
        "List of instrumented functions should contain {stuff:?}. Complete list is {list:?}"
    );

    assert!(
        all.is_empty(),
        "the complete list of functions should be empty, nothing is defined in this file. Got this instead: {all:?}"
    );
}

#[test]
fn detect_two_args_wrapper() {
    let source = r#"
  import { autometrics } from "autometrics";

  const getWow = autometrics(
    {
      functionName: "getThatWow",
      moduleName: "MODULE",
    },
    async () => {
      const res = await fetch(
        "https://owen-wilson-wow-api.onrender.com/wows/random"
      );
      return await res.json();
    }
  );
        "#;

    let list = AmQuery::try_new()
        .unwrap()
        .list_function_names(FILE_NAME, MODULE_NAME, source, None)
        .unwrap();
    let all = AllFunctionsQuery::try_new()
        .unwrap()
        .list_function_names(FILE_NAME, MODULE_NAME, source)
        .unwrap();
    let get_wow_location = Location {
        file: FILE_NAME.to_string(),
        range: Range {
            start: Position {
                line: 5,
                column: 21,
            },
            end: Position {
                line: 5,
                column: 21 + "getThatWow".len(),
            },
        },
    };

    let get_wow = FunctionInfo {
        id: ("MODULE", "getThatWow").into(),
        instrumentation: Some(get_wow_location),
        // TODO: getWow is instrumented using the wrapper function,
        // therefore AmQuery::list_function_names is not expected to guess that the definition is here as well
        //
        // But maybe it should, since the function is defined in place? It's low priority, because
        // AllFunctionsQuery is supposed to catch the definition and eventually we want to merge the
        // lists.
        definition: None,
    };

    assert_eq!(
        list.len(),
        1,
        "list should have 1 item, got this instead: {list:?}"
    );
    assert_eq!(list[0], get_wow);

    assert_eq!(
        all.len(),
        0,
        "list of all functions should have 0 items, got this instead: {all:?}"
    );
}
