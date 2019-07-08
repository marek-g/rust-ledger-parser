# ledger-parser

[![Crates.io Version](https://img.shields.io/crates/v/ledger-parser.svg)](https://crates.io/crates/ledger-parser)
[![Docs.rs Version](https://docs.rs/ledger-parser/badge.svg)](https://docs.rs/ledger-parser)
[![License Unlicense](https://img.shields.io/crates/l/ledger-parser.svg)](http://unlicense.org/UNLICENSE)

Rust library for parsing [Ledger-cli](https://www.ledger-cli.org/) input files.

## File format

Only a subset of the ledger-cli's file format is implemented.

Supported elements:

* Line comments (starting with: ``; # % | *``)

* Inline comments (starting with ``;``)

* Transaction headers with format:

  ```
  DATE[=EDATE] [*|!] [(CODE)] DESC
  ```

* Transaction postings with format (minimum two spaces or one tab between ``ACCOUNT`` and ``AMOUNT``):

  ```
    ACCOUNT  [AMOUNT] [= BALANCE] [; NOTE]
  ```

  There may be only a single posting without an amount or balance in a transaction.

* Commodity prices with format:

  ```
  P DATE SYMBOL PRICE
  ```

## Example

```rust
extern crate ledger_parser;

let result = ledger_parser::simple::parse(r#"; Example 1
2018-10-01=2018-10-14 ! (123) Description
  ; Transaction comment
  TEST:Account 123  $1.20
  ; Posting comment
  TEST:Account 345  -$1.20"#);
```

## Two types of AST trees

There are two types of AST trees you can get with this library:

### ast::Ledger

The `ast::Ledger` tree is parsed with `ast::parse()` method. It represents closely the original content keeping such information as:
- information about empty amount fields
- balance verification data
- original order of entries (transactions and commodity prices)

Additionally it does only a simple verification (datetime format, missing amount fields).

This tree type is recommended to use with application types like format converters or data fetchers which wants to append new entries etc.

### simple::Ledger

The `simple::Ledger` tree is parsed with `simple::parse()` method. It calls additional transformation phase after the `ast::parse()`. During that phase the tree is simplified. The result doesn't contain such data as optional amount fields or balance verification data. These are replaced with the correct calculated amount (or an error is reported).

This tree is easier to start working with for application types like report generators etc.
