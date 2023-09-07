# Autometrics List

A command that lists all functions that have the "autometrics" annotation.

The aim is to use this binary as a quick static analyzer that returns from a
codebase the complete list of functions that are annotated to be
autometricized.

The analysis is powered by [Tree-sitter](https://tree-sitter.github.io), and
all the specific logic is contained in [Tree-sitter queries](./runtime/queries)
that are specific for each language implementation.

## Quickstart

Use the installer script to pull the latest version directly from Github
(change `VERSION` accordingly):

```console
VERSION=0.2.0 curl --proto '=https' --tlsv1.2 -LsSf https://github.com/autometrics-dev/am_list/releases/download/v$VERSION/am_list-installer.sh | sh
```

And run the binary

```bash
# Make sure that `~/.cargo/bin` is in your `PATH`
am_list list -l rs /path/to/project/root
```

## Current state and known issues

### Language support table

In the following table, having the "detection" feature means that `am_list`
returns the exact same labels as the ones you would need to use in PromQL to
look at the metrics. In a nutshell,
"[Autometrics](https://github.com/autometrics-dev) compliance".

|                            Language                             | Function name detection | Module detection |
| :-------------------------------------------------------------: | :---------------------: | :--------------: |
|    [Rust](https://github.com/autometrics-dev/autometrics-rs)    |           ✅            |        ✅        |
| [Typescript](https://github.com/autometrics-dev/autometrics-ts) |           ✅            |   ⚠️[^wrapper]   |
|     [Go](https://github.com/autometrics-dev/autometrics-go)     |   ⚠️[^all-functions]    |        ✅        |
|   [Python](https://github.com/autometrics-dev/autometrics-py)   |           ✅            |        ✅        |
|     [C#](https://github.com/autometrics-dev/autometrics-cs)     |           ❌            |        ❌        |

[^wrapper]:
    For Typescript (and all languages where autometrics is a wrapper
    function), static analysis makes it hard to traverse imports to find the
    module where an instrumented function is _defined_, so the reported module
    is the module where the function has been _instrumented_

[^all-functions]:
    Support list all autometricized functions, but not all
    functions without restriction

### Typescript

#### Module tracking

This tool cannot track modules "accurately" (meaning "the module label is
exactly what autometrics will report"), because autometrics-ts uses the path of
the source in the JS-compiled bundle to report the module. The compilation and
bundling happens after `am_list` looks at the code so it cannot be accurate.

This means the module reporting for typescript is bound to be a "best effort"
attempt to be useful.

The other difficulty encountered when using a static analysis tool with autometrics-ts is that the
instrumentation can happen anywhere, as the wrapper function call can use an imported symbol as its argument:

```typescript
import { exec } from "child_process";
import { autometrics } from "@autometrics/autometrics";

const instrumentedExec = autometrics(exec);

// use instrumentedExec everywhere instead of exec
```

In order to report the locus of _function definition_ as the module, we would
need to include both:

- a complete import resolution step, to figure out the origin module of the
  instrumented function (`child_process` in the example), and
- a dependency inspection step, to figure out the path to the instrumented
  function definition _within_ the dependency (`lib/child_process.js` in the
  [node source code](https://github.com/nodejs/node/blob/main/lib/child_process.js))

This is impractical and error-prone to implement these steps accurately, so
instead we only try to detect imports when they are explicitely imported in the
same file, and we will only report the function module as the imported module
(not the path to the file it is defined in). Practically that means that for
this example:

```typescript
// in src/router/index.ts
import { exec } from "child_process";
import { origRoute as myRoute } from "../handlers";
import { autometrics } from "@autometrics/autometrics";

const instrumentedExec = autometrics(exec);
const instrumentedRoute = autometrics(myRoute);

// use instrumentedExec everywhere instead of exec
```

`am_list` will report 2 functions:

- `{"function": "exec", "module": "ext://child_process"}`: using `ext://`
  protocol to say the module is non-local
- `{"function": "origRoute", "module": "handlers"}`: even if `myRoute` is
  re-exported from `../handlers/my/routes/index.ts`, we do not go look into what
  `handlers` did to expose `origRoute`; also, the alias is resolved.
