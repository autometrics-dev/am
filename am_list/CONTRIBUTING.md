# Contributing

## Implementing language support

To add `am_list` support for a new language, you mostly need to know the usage
patterns of autometrics in this language, and understand how to make and use
tree-sitter queries.

### Function detection architecture

The existing implementations can also be used as a stencil to add a new
language. They all follow the same pattern:
- a `src/{lang}.rs` which implements walking down a directory and collecting the
`ExpectedAmLabel` labels on the files it traverses. This is the location where
the `crate::ListAmFunctions` trait is implemented.
- a `src/{lang}/queries.rs` which is a wrapper around a tree-sitter query, using
  the rust bindings.
- a `runtime/queries/{lang}` folder which contains the actual queries being
  passed to tree-sitter:
  + the `.scm` files are direct queries, ready to be used "as-is",
  + the `.scm.tpl` files are format strings, which are meant to be templated at
    runtime according to other detection happening in the file. For example,
    when autometrics is a wrapper function that can be renamed in a file
    (`import { autometrics as am } from '@autometrics/autometrics'` in
    typescript), we need to change the exact query to detect wrapper usage in
    the file.
    
### Building queries

The hardest part about supporting a language is to build the queries for it. The
best resources to create those are on the main website of tree-sitter library:

- the help section on [Query
  syntax](https://tree-sitter.github.io/tree-sitter/using-parsers#query-syntax)
  helps understand how to create queries, and
- the [playground](https://tree-sitter.github.io/tree-sitter/playground) allows
  to paste some text and see both the syntax tree and the current highlight from
  a query. This enables creating queries incrementally.

![Screenshot of the Tree-sitter playground on their website](./assets/contributing/ts-playground.png)
  
If you're using Neovim, it also has a very good [playground
plugin](https://github.com/nvim-treesitter/playground) you can use.

The goal is to test the query on the common usage patterns of autometrics, and
make sure that you can match the function names. There are non-trivial
limitations about what a tree-sitter query can and cannot match, so if you're
blocked, don't hesitate to create a discussion or an issue to ask a question,
and we will try to solve the problem together.

### Using queries

Once you have you query, it's easier to wrap it in a Rust structure to keep
track of the indices of the "captures" created within the query. It is best to
look at how the queries are used in the existing implementations and try to
replicate this, seeing how the bindings are used. In order of complexity, you
should look at:
- the [Go](./src/go/queries.rs) implementation, which has a very simple query.
- the [Typescript](./src/typescript/queries.rs) implementation, which uses the
  result of a first query to dynamically create a second one, and collect the
  results of everything.
- the [Rust](./src/rust/queries.rs) implementation, which uses dynamically
  created queries, but also uses a recursion pattern to keep track of modules
  declared in-file.
