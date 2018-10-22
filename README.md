# ledger-parser

[![Crates.io Version](https://img.shields.io/crates/v/ledger-parser.svg)](https://crates.io/crates/ledger-parser)

Rust library for parsing ledger cli (https://www.ledger-cli.org/) input files.

## Example

```rust
extern crate ledger_parser;

let result = ledger_parser::parse(r#"; Example 1
2018-10-01=2018-10-14 ! (123) Marek Ogarek
  TEST:ABC 123  $1.20
  TEST:ABC 123  $1.20"#);
```
