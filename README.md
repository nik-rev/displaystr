# `displaystr`

<!-- cargo-rdme start -->

[![crates.io](https://img.shields.io/crates/v/displaystr?style=flat-square&logo=rust)](https://crates.io/crates/displaystr)
[![docs.rs](https://img.shields.io/badge/docs.rs-displaystr-blue?style=flat-square&logo=docs.rs)](https://docs.rs/displaystr)
![license](https://img.shields.io/badge/license-Apache--2.0_OR_MIT-blue?style=flat-square)
![msrv](https://img.shields.io/badge/msrv-1.56-blue?style=flat-square&logo=rust)
[![github](https://img.shields.io/github/stars/nik-rev/displaystr)](https://github.com/nik-rev/displaystr)

This crate provides a convenient attribute macro that implements [`Display`](https://doc.rust-lang.org/stable/core/fmt/trait.Display.html) for you

```toml
[dependencies]
displaystr = "0.1"
```

**Bonus:** This crate has 0 dependencies. I think compile-times are very important, so I have put a lot of effort into optimizing them.

## Example

Apply `#[display]` on `enum`s:

```rust
use displaystr::display;

#[display]
pub enum DataStoreError {
    Disconnect(std::io::Error) = "data store disconnected",
    Redaction(String) = "the data for key `{_0}` is not available",
    InvalidHeader {
        expected: String,
        found: String,
    } = "invalid header (expected {expected:?}, found {found:?})",
    Unknown = "unknown data store error",
}
```

The above expands to this:

```rust
use displaystr::display;

pub enum DataStoreError {
    Disconnect(std::io::Error),
    Redaction(String),
    InvalidHeader {
        expected: String,
        found: String,
    },
    Unknown,
}

impl ::core::fmt::Display for DataStoreError {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self {
            Self::Disconnect(_0) => {
                f.write_fmt(format_args!("data store disconnected"))
            }
            Self::Redaction(_0) => {
                f.write_fmt(format_args!("the data for key `{_0}` is not available"))
            }
            Self::InvalidHeader { expected, found } => {
                f.write_fmt(format_args!("invalid header (expected {expected}, found {found})"))
            }
            Self::Unknown => {
                f.write_fmt(format_args!("unknown data store error"))
            }
        }
    }
}
```

## Auto-generated doc comments

Use `#[display(doc)]` to automatically generate `///` comments. The above example's expansion `enum` would generate this:

```rust
use displaystr::display;

pub enum DataStoreError {
    /// data store disconnected
    Disconnect(std::io::Error),
    /// the data for key `{_0}` is not available
    Redaction(String),
    /// invalid header (expected {expected:?}, found {found:?})
    InvalidHeader {
        expected: String,
        found: String,
    },
    /// unknown data store error
    Unknown,
}
```

## Multiple arguments

You can use a tuple to supply multiple argumenst to the `format_args!`:

```rust
use displaystr::display;

#[display]
pub enum DataStoreError {
    Redaction(String, Vec<String>) = ("the data for key `{_0}` is not available, but we recovered: {}", _1.join("+")),
}
```

Expands to this:

```rust
use displaystr::display;

pub enum DataStoreError {
    Redaction(String, Vec<String>),
}

impl ::core::fmt::Display for DataStoreError {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self {
            Self::Redaction(_0, _1) => {
                f.write_fmt(format_args!("the data for key `{_0}` is not available, but we recovered: {}", _1.join("+")))
            }
        }
    }
}
```

<!-- cargo-rdme end -->
