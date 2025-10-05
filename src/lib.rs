//! [![crates.io](https://img.shields.io/crates/v/displaystr?style=flat-square&logo=rust)](https://crates.io/crates/displaystr)
//! [![docs.rs](https://img.shields.io/badge/docs.rs-displaystr-blue?style=flat-square&logo=docs.rs)](https://docs.rs/displaystr)
//! ![license](https://img.shields.io/badge/license-Apache--2.0_OR_MIT-blue?style=flat-square)
//! ![msrv](https://img.shields.io/badge/msrv-1.56-blue?style=flat-square&logo=rust)
//! [![github](https://img.shields.io/github/stars/nik-rev/displaystr)](https://github.com/nik-rev/displaystr)
//!
//! This crate provides a convenient attribute macro that implements [`Display`](core::fmt::Display) for you
//!
//! ```toml
//! [dependencies]
//! displaystr = "0.1"
//! ```
//!
//! This crate has 0 dependencies. I think compile-times are very important, so I have put a lot of effort into optimizing them.
//!
//! # Example
//!
//! Apply [`#[display]`](display) on `enum`s:
//!
//! ```rust
//! use displaystr::display;
//!
//! #[display]
//! pub enum DataStoreError {
//!     Disconnect(std::io::Error) = "data store disconnected",
//!     Redaction(String) = "the data for key `{_0}` is not available",
//!     InvalidHeader {
//!         expected: String,
//!         found: String,
//!     } = "invalid header (expected {expected:?}, found {found:?})",
//!     Unknown = "unknown data store error",
//! }
//! ```
//!
//! The above expands to this:
//!
//! ```rust
//! use displaystr::display;
//!
//! pub enum DataStoreError {
//!     Disconnect(std::io::Error),
//!     Redaction(String),
//!     InvalidHeader {
//!         expected: String,
//!         found: String,
//!     },
//!     Unknown,
//! }
//!
//! impl ::core::fmt::Display for DataStoreError {
//!     fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
//!         match self {
//!             Self::Disconnect(_0) => {
//!                 f.write_fmt(format_args!("data store disconnected"))
//!             }
//!             Self::Redaction(_0) => {
//!                 f.write_fmt(format_args!("the data for key `{_0}` is not available"))
//!             }
//!             Self::InvalidHeader { expected, found } => {
//!                 f.write_fmt(format_args!("invalid header (expected {expected}, found {found})"))
//!             }
//!             Self::Unknown => {
//!                 f.write_fmt(format_args!("unknown data store error"))
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! # Auto-generated doc comments
//!
//! Use `#[display(doc)]` to automatically generate `///` comments. The above example's expansion `enum` would generate this:
//!
//! ```rust
//! use displaystr::display;
//!
//! pub enum DataStoreError {
//!     /// data store disconnected
//!     Disconnect(std::io::Error),
//!     /// the data for key `{_0}` is not available
//!     Redaction(String),
//!     /// invalid header (expected {expected:?}, found {found:?})
//!     InvalidHeader {
//!         expected: String,
//!         found: String,
//!     },
//!     /// unknown data store error
//!     Unknown,
//! }
//! ```
//!
//! # Multiple arguments
//!
//! You can use a tuple to supply multiple arguments to the `format_args!`:
//!
//! ```rust
//! use displaystr::display;
//!
//! #[display]
//! pub enum DataStoreError {
//!     Redaction(String, Vec<String>) = (
//!         "the data for key `{_0}` is not available, but we recovered: {}",
//!         _1.join("+"),
//!     ),
//! }
//! ```
//!
//! Expands to this:
//!
//! ```rust
//! use displaystr::display;
//!
//! pub enum DataStoreError {
//!     Redaction(String, Vec<String>),
//! }
//!
//! impl ::core::fmt::Display for DataStoreError {
//!     fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
//!         match self {
//!             Self::Redaction(_0, _1) => f.write_fmt(format_args!(
//!                 "the data for key `{_0}` is not available, but we recovered: {}",
//!                 _1.join("+")
//!             )),
//!         }
//!     }
//! }
//! ```

use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

/// Ergonomically implement [`Display`](::core::fmt::Display) for `enum`s
///
/// # Example
///
/// ```rust
/// use displaystr::display;
///
/// #[display]
/// pub enum DataStoreError {
///     Disconnect(std::io::Error) = "data store disconnected",
///     InvalidHeader {
///         expected: String,
///         found: String,
///     } = "invalid header (expected {expected:?}, found {found:?})",
/// }
/// ```
///
/// The above expands to this:
///
/// ```rust
/// use displaystr::display;
///
/// pub enum DataStoreError {
///     Disconnect(std::io::Error),
///     InvalidHeader {
///         expected: String,
///         found: String,
///     },
/// }
///
/// impl ::core::fmt::Display for DataStoreError {
///     fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
///         match self {
///             Self::Disconnect(_0) => {
///                 f.write_fmt(format_args!("data store disconnected"))
///             }
///             Self::InvalidHeader { expected, found } => {
///                 f.write_fmt(format_args!("invalid header (expected {expected}, found {found})"))
///             }
///         }
///     }
/// }
/// ```
///
/// For more information, see the [crate-level](crate) documentation
#[proc_macro_attribute]
pub fn display(args: TokenStream, ts: TokenStream) -> TokenStream {
    // Contains all `compile_error!("msg")` which we'll report all at once
    let mut compile_errors = TokenStream::new();

    let mut args = args.into_iter();

    let generate_doc_comments = match args.next() {
        Some(TokenTree::Ident(ident)) if ident.to_string() == "doc" => {
            if let Some(next) = args.next() {
                compile_errors.extend(CompileError::new(next.span(), "unexpected token"));
            }
            true
        }
        Some(tt) => {
            compile_errors.extend(CompileError::new(tt.span(), "unexpected token"));
            false
        }
        None => false,
    };

    // This is the final output that we'll emit.
    // It's the same, but we are gonna strip all the discriminant strings

    let mut output = TokenStream::new();
    let mut ts = ts.into_iter().peekable();

    // Parse + ignore everything until and including the `enum` keyword
    //
    // #[foo = bar] pub(crate) enum Foo { ... }
    // ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ skip all of this
    loop {
        let Some(tt) = ts.next() else {
            return CompileError::new(Span::call_site(), "expected an `enum` item")
                .into_iter()
                .collect();
        };

        match tt {
            // Skip all `OuterAttribute`
            //
            // #[foo = bar]
            // ^
            TokenTree::Punct(punct) if punct == '#' => {
                output.extend([TokenTree::Punct(punct)]);

                // #[foo = bar]
                //  ^^^^^^^^^^^
                output.extend(ts.next());
            }
            // Reached enum keyword. End parsing.
            //
            // enum Foo { ... }
            // ^^^^
            TokenTree::Ident(ident) if ident.to_string() == "enum" => {
                output.extend([TokenTree::Ident(ident)]);
                break;
            }
            // ignore any other token e.g. `pub` or `(crate)`
            tt => {
                output.extend([tt]);
            }
        }
    }

    // enum Foo { ... }
    //     ^ we are here now

    let enum_ident = match ts.next() {
        Some(TokenTree::Ident(ident)) => ident,
        _ => unreachable!("`enum` is always followed by an identifier"),
    };

    // enum Foo <all: of_the_generics> { ... }
    //         ^ we are here now

    // enum Foo <all: of_the_generics> { ... }
    //          ^^^^^^^^^^^^^^^^^^^^^^ contains all generics on the item
    let generics = match ts.peek() {
        // opening '<'
        //
        // enum Foo <all: of_the_generics> { ... }
        //          ^
        Some(TokenTree::Punct(punct)) if *punct == '<' => {
            let mut generics = TokenStream::new();
            generics.extend(ts.next());

            loop {
                match ts.next() {
                    // closing '>'
                    //
                    // enum Foo <all: of_the_generics> { ... }
                    //                               ^
                    Some(TokenTree::Punct(punct)) if punct == '>' => {
                        generics.extend([TokenTree::Punct(punct)]);
                        break;
                    }
                    tt => {
                        generics.extend(tt);
                    }
                }
            }

            generics
        }
        _ => TokenStream::new(),
    };

    // enum Foo where A: B { ... }
    //          ^^^^^^^^^^ contains the entire where clause
    let where_clause = match ts.peek() {
        Some(TokenTree::Ident(ident)) if ident.to_string() == "where" => {
            let mut where_clause = TokenStream::new();
            where_clause.extend(ts.next());

            loop {
                match ts.peek() {
                    Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Brace => break,
                    _ => {
                        where_clause.extend(ts.next());
                    }
                }
            }

            where_clause
        }
        _ => TokenStream::new(),
    };

    // enum Foo where A: B { ... }
    //                    ^ we are here now

    // enum Foo where A: B { ... }
    //                     ^^^^^^^ contains all of the variants
    let mut enum_body = match ts.next() {
        Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Brace => {
            group.stream().into_iter().peekable()
        }
        _ => unreachable!("enum has braces"),
    };

    // enum Foo <all: of_the_generics> { ... }
    //                                ^ we are here now

    // All enum variants, exactly as-is just with the string discriminant removed
    //
    // We do this because we're not a `#[derive()]` macro. We are an attribute macro -
    // but we don't really want to change the original input all that much.
    let mut variants = TokenStream::new();

    // All arms of the `match` generated inside the `Display` impl
    let mut arms = TokenStream::new();

    // Each iteration of this loop parses a single variant
    //
    // enum Foo {
    //     Bar(u32) = "bar",
    //     ^^^^^^^^^^^^^^^^^
    //     Baz = "foo"
    //     ^^^^^^^^^^^
    // }
    loop {
        if enum_body.peek().is_none() {
            break;
        }

        // Parse all attributes on the variant

        loop {
            match enum_body.peek() {
                Some(TokenTree::Punct(punct)) if *punct == '#' => {
                    // #[foo = bar]
                    // ^
                    variants.extend(enum_body.next());
                    // #[foo = bar]
                    //  ^^^^^^^^^^^
                    variants.extend(enum_body.next());
                }
                // no more attributes
                _ => break,
            }
        }

        // We'll append rest of the variant in here, because
        // if we generate doc comments we'll want to add them at the end
        //
        // #[doc = "bar"]
        // ^^^^^^^^^^^^^^ we want to generate this if we were called with `display(doc)`
        //
        // Foo = "bar",
        //       ^^^^^ give this
        let mut variant = TokenStream::new();

        // Parse visibility of the variant (semantically rejected, but syntactically valid)

        // pub(crate) Foo
        // ^^^^^^^^^^
        match enum_body.peek() {
            // pub(crate)
            // ^^^
            Some(TokenTree::Ident(ident)) if ident.to_string() == "pub" => {
                variant.extend(enum_body.next());

                match enum_body.peek() {
                    // pub(in crate)
                    //    ^^^^^^^^^^
                    Some(TokenTree::Group(group))
                        if group.delimiter() == Delimiter::Parenthesis =>
                    {
                        variant.extend(enum_body.next());
                    }
                    _ => (),
                }
            }
            _ => (),
        }

        // Variant identifier
        //
        // Foo {}
        // ^^^

        let variant_ident = match enum_body.next() {
            Some(TokenTree::Ident(ident)) => {
                variant.extend([TokenTree::Ident(ident.clone())]);
                ident
            }
            _ => unreachable!("identifier must appear in this position"),
        };

        // Foo { a: usize, b: usize }
        //     ^^^^^^^^^^^^^^^^^^^^^^
        match enum_body.next() {
            // tuple variant
            //
            // Foo(a, b) = "foo",
            //    ^^^^^^
            Some(TokenTree::Group(fields)) if fields.delimiter() == Delimiter::Parenthesis => {
                variant.extend([TokenTree::Group(fields.clone())]);

                // Foo() has 0 variant. Otherwise, start the count at 1
                let mut fields = fields.stream().into_iter().peekable();
                let is_zero_variants = fields.peek().is_none();

                // Let's count how many commas there are between each field
                //
                // Foo(bar, baz, quux,)
                //        ^ this one
                //             ^ and this one
                //                   ^ but not this one (it is trailing)
                let mut commas = 0;

                while let Some(tt) = fields.next() {
                    match tt {
                        TokenTree::Punct(punct) if punct == ',' => {
                            // Only count non-trailing commas
                            if fields.peek().is_some() {
                                commas += 1;
                            }
                        }
                        // ignore any other tokens
                        _ => (),
                    }
                }

                // Foo() has 0 fields
                // Foo(bar) has 1 field
                // Foo(bar,) has 1 field
                // Foo(bar,baz) has 2 fields
                let variant_count = if is_zero_variants { 0 } else { 1 + commas };

                // Self::Disconnect(_0, _1) => f.write_fmt(format_args!("..."))
                //                  ^^^^^^
                let destructure = (0..variant_count)
                    .flat_map(|i| {
                        [
                            TokenTree::Ident(Ident::new(&format!("_{i}"), Span::call_site())),
                            TokenTree::Punct(Punct::new(',', Spacing::Joint)),
                        ]
                    })
                    .collect();

                // Self::Disconnect(_0, _1) => f.write_fmt(format_args!("..."))
                //                 ^^^^^^^^
                let destructure = TokenTree::Group(Group::new(Delimiter::Parenthesis, destructure));

                // Foo(a, b) = "foo",
                //           ^^^^^^^
                match extract_eq_string(&mut enum_body, variant_ident.span()) {
                    Ok((string, stream)) => {
                        if generate_doc_comments {
                            variants.extend(doc_comment(&string.to_string()));
                        }
                        arms.extend(generate_arm(
                            &variant_ident.to_string(),
                            destructure,
                            string,
                            stream,
                        ));
                    }
                    Err(compile_error) => {
                        // dummy arm so we just continue compiling
                        arms.extend(generate_arm(
                            &variant_ident.to_string(),
                            destructure,
                            Literal::string(""),
                            TokenStream::new(),
                        ));
                        compile_errors.extend(compile_error);
                    }
                };

                // Foo(a, b) = "foo",
                //                  ^
                match enum_body.peek() {
                    Some(TokenTree::Punct(punct)) if *punct == ',' => {
                        // trailing comma
                        variant.extend(enum_body.next());
                    }
                    _ => (),
                }
            }
            // struct variant
            //
            // Foo { a: bool, b: usize } = "foo"
            //     ^^^^^^^^^^^^^^^^^^^^^
            Some(TokenTree::Group(fields)) if fields.delimiter() == Delimiter::Brace => {
                variant.extend([TokenTree::Group(fields.clone())]);

                // if we are after `:`. We hold this because types themselves can contain `:`, e.g
                // `for<T: Bar> Foo<T>`
                let mut is_inside_type = false;

                let mut fields = fields.stream().into_iter().peekable();

                // Self::InvalidHeader { expected, found, } => f.write_fmt(format_args!("..."))
                //                       ^^^^^^^^^^^^^^^^
                let mut destructure = TokenStream::new();

                // Obtain all the fields. Every field has `:` preceded by an identifier.
                loop {
                    let current = fields.next();
                    match fields.peek() {
                        Some(TokenTree::Punct(punct)) if *punct == ':' && !is_inside_type => {
                            // Self::InvalidHeader { expected, found, } => f.write_fmt(format_args!("..."))
                            //                       ^^^^^^^^
                            destructure.extend(current);
                            // Self::InvalidHeader { expected, found, } => f.write_fmt(format_args!("..."))
                            //                               ^
                            destructure.extend([TokenTree::Punct(Punct::new(',', Spacing::Joint))]);
                            is_inside_type = true;
                        }
                        // foo: Bar,
                        //         ^
                        Some(TokenTree::Punct(punct)) if *punct == ',' && is_inside_type => {
                            is_inside_type = false;
                        }
                        // ignore any other tokens like `#[doc = "..."]` or `pub(crate)`
                        Some(_) => (),
                        // Reached end of the struct variant's fields
                        None => break,
                    }
                }

                // Self::InvalidHeader { expected, found, } => f.write_fmt(format_args!("..."))
                //                     ^^^^^^^^^^^^^^^^^^^^
                let destructure = TokenTree::Group(Group::new(Delimiter::Brace, destructure));

                // Foo { a: bool, b: usize } = "foo",
                //                           ^^^^^^^
                match extract_eq_string(&mut enum_body, variant_ident.span()) {
                    Ok((string, stream)) => {
                        if generate_doc_comments {
                            variants.extend(doc_comment(&string.to_string()));
                        }
                        arms.extend(generate_arm(
                            &variant_ident.to_string(),
                            destructure,
                            string,
                            stream,
                        ));
                    }
                    Err(compile_error) => {
                        // dummy arm so we just continue compiling
                        arms.extend(generate_arm(
                            &variant_ident.to_string(),
                            destructure,
                            Literal::string(""),
                            TokenStream::new(),
                        ));
                        compile_errors.extend(compile_error);
                    }
                };

                // Foo { a: bool, b: usize } = "foo",
                //                                  ^
                match enum_body.peek() {
                    Some(TokenTree::Punct(punct)) if *punct == ',' => {
                        // trailing comma
                        variant.extend(enum_body.next());
                    }
                    _ => (),
                }
            }
            // unit variant with discriminant after it
            //
            // Foo = "foo",
            //     ^
            Some(TokenTree::Punct(punct)) if punct == '=' => {
                // Foo = "foo",
                //       ^^^^^
                match extract_string(&mut enum_body) {
                    Ok((string, stream)) => {
                        if generate_doc_comments {
                            variants.extend(doc_comment(&string.to_string()));
                        }
                        // Success.
                        arms.extend(generate_arm(
                            &variant_ident.to_string(),
                            // Foo {}
                            //     ^^
                            TokenTree::Group(Group::new(Delimiter::Brace, TokenStream::new())),
                            string,
                            stream,
                        ));
                    }
                    Err(compile_error) => {
                        compile_errors.extend(compile_error);

                        // DUMMY arm so we compile. so rust-analyzer works better
                        arms.extend(generate_arm(
                            &variant_ident.to_string(),
                            // Foo {}
                            //     ^^
                            TokenTree::Group(Group::new(Delimiter::Brace, TokenStream::new())),
                            Literal::string(""),
                            TokenStream::new(),
                        ));
                    }
                }

                // Foo = "foo",
                //            ^
                match enum_body.peek() {
                    Some(TokenTree::Punct(punct)) if *punct == ',' => {
                        // trailing comma
                        variant.extend(enum_body.next());
                    }
                    _ => (),
                }
            }
            // unit variant with comma after it (invalid)
            //
            // Foo,
            //    ^
            Some(TokenTree::Punct(punct)) if punct == ',' => {
                variant.extend([TokenTree::Punct(punct.clone())]);

                compile_errors.extend(CompileError::new(
                    variant_ident.span(),
                    "expected this variant to have a string discriminant: `= \"...\"`",
                ));

                // DUMMY arm so we compile. so rust-analyzer works better
                arms.extend(generate_arm(
                    &variant_ident.to_string(),
                    // Foo {}
                    //     ^^
                    TokenTree::Group(Group::new(Delimiter::Brace, TokenStream::new())),
                    Literal::string(""),
                    TokenStream::new(),
                ));
            }
            // unit variant with no comma after it (it the last variant)
            //
            // Foo
            //    ^
            None => {
                compile_errors.extend(CompileError::new(
                    variant_ident.span(),
                    "expected this variant to have a string discriminant: `= \"...\"`",
                ));

                // DUMMY arm so we compile. so rust-analyzer works better
                arms.extend(generate_arm(
                    &variant_ident.to_string(),
                    // Foo {}
                    //     ^^
                    TokenTree::Group(Group::new(Delimiter::Brace, TokenStream::new())),
                    Literal::string(""),
                    TokenStream::new(),
                ));

                break;
            }
            Some(_) => unreachable!("no other token is valid in this position"),
        }

        variants.extend(variant);
    }

    // The original enum. Re-constructed but without the string discriminants
    let original_enum = output
        .into_iter()
        .chain([TokenTree::Ident(enum_ident.clone())])
        .chain(generics)
        .chain(where_clause)
        .chain([TokenTree::Group(Group::new(Delimiter::Brace, variants))]);

    // actual implementation of the `Display` trait
    //
    // equivalent to:
    //
    // quote! {
    //     impl ::core::fmt::Display for #enum_ident {
    //         fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
    //             match self {
    //                 #arms
    //             }
    //         }
    //     }
    // }
    let display_impl = TokenStream::from_iter([
        TokenTree::Ident(Ident::new("impl", Span::call_site())),
        TokenTree::Punct(Punct::new(':', Spacing::Joint)),
        TokenTree::Punct(Punct::new(':', Spacing::Joint)),
        TokenTree::Ident(Ident::new("core", Span::call_site())),
        TokenTree::Punct(Punct::new(':', Spacing::Joint)),
        TokenTree::Punct(Punct::new(':', Spacing::Joint)),
        TokenTree::Ident(Ident::new("fmt", Span::call_site())),
        TokenTree::Punct(Punct::new(':', Spacing::Joint)),
        TokenTree::Punct(Punct::new(':', Spacing::Joint)),
        TokenTree::Ident(Ident::new("Display", Span::call_site())),
        TokenTree::Ident(Ident::new("for", Span::call_site())),
        TokenTree::Ident(enum_ident),
        TokenTree::Group(Group::new(
            Delimiter::Brace,
            TokenStream::from_iter([
                TokenTree::Ident(Ident::new("fn", Span::call_site())),
                TokenTree::Ident(Ident::new("fmt", Span::call_site())),
                TokenTree::Group(Group::new(
                    Delimiter::Parenthesis,
                    TokenStream::from_iter([
                        TokenTree::Punct(Punct::new('&', Spacing::Joint)),
                        TokenTree::Ident(Ident::new("self", Span::call_site())),
                        TokenTree::Punct(Punct::new(',', Spacing::Joint)),
                        TokenTree::Ident(Ident::new("f", Span::call_site())),
                        TokenTree::Punct(Punct::new(':', Spacing::Joint)),
                        TokenTree::Punct(Punct::new('&', Spacing::Joint)),
                        TokenTree::Ident(Ident::new("mut", Span::call_site())),
                        TokenTree::Punct(Punct::new(':', Spacing::Joint)),
                        TokenTree::Punct(Punct::new(':', Spacing::Joint)),
                        TokenTree::Ident(Ident::new("core", Span::call_site())),
                        TokenTree::Punct(Punct::new(':', Spacing::Joint)),
                        TokenTree::Punct(Punct::new(':', Spacing::Joint)),
                        TokenTree::Ident(Ident::new("fmt", Span::call_site())),
                        TokenTree::Punct(Punct::new(':', Spacing::Joint)),
                        TokenTree::Punct(Punct::new(':', Spacing::Joint)),
                        TokenTree::Ident(Ident::new("Formatter", Span::call_site())),
                    ]),
                )),
                TokenTree::Punct(Punct::new('-', Spacing::Joint)),
                TokenTree::Punct(Punct::new('>', Spacing::Joint)),
                TokenTree::Punct(Punct::new(':', Spacing::Joint)),
                TokenTree::Punct(Punct::new(':', Spacing::Joint)),
                TokenTree::Ident(Ident::new("core", Span::call_site())),
                TokenTree::Punct(Punct::new(':', Spacing::Joint)),
                TokenTree::Punct(Punct::new(':', Spacing::Joint)),
                TokenTree::Ident(Ident::new("fmt", Span::call_site())),
                TokenTree::Punct(Punct::new(':', Spacing::Joint)),
                TokenTree::Punct(Punct::new(':', Spacing::Joint)),
                TokenTree::Ident(Ident::new("Result", Span::call_site())),
                TokenTree::Group(Group::new(
                    Delimiter::Brace,
                    TokenStream::from_iter([
                        TokenTree::Ident(Ident::new("match", Span::call_site())),
                        TokenTree::Ident(Ident::new("self", Span::call_site())),
                        TokenTree::Group(Group::new(Delimiter::Brace, arms)),
                    ]),
                )),
            ]),
        )),
    ]);

    original_enum
        .chain(compile_errors)
        .chain(display_impl)
        .collect()
}

/// Given a `ts` which contains `= "..."`, extract it and return as `DisplayArm`
///
/// ```ignore
/// Self::InvalidHeader { expected, found, } => f.write_fmt(format_args!("..."))
///       ^^^^^^^^^^^^^ variant_ident
///                     ^^^^^^^^^^^^^^^^^^^^ destructure
///                                                                      ^^^^^ string
/// ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ return (DisplayArm)
/// ```
#[allow(clippy::result_large_err)]
fn extract_eq_string(
    ts: &mut std::iter::Peekable<proc_macro::token_stream::IntoIter>,
    variant_ident_span: Span,
) -> Result<(Literal, TokenStream), CompileError> {
    // NOTE: We nest it because even if there is no discriminant (`= "foo"`) we still want to
    // output a syntactically valid enum so rust-analyzer can work with it for better DX
    match ts.next() {
        Some(TokenTree::Punct(punct)) if punct == '=' => {
            // Foo { a: bool, b: usize } = "foo"
            //                             ^^^^^
            extract_string(ts)
        }
        _ => Err(CompileError::new(
            variant_ident_span,
            "expected this variant to have a string discriminant: `= \"...\"`",
        )),
    }
}

/// Generates a doc comment `///`
fn doc_comment(content: &str) -> [TokenTree; 2] {
    [
        TokenTree::Punct(Punct::new('#', Spacing::Joint)),
        TokenTree::Group(Group::new(
            Delimiter::Bracket,
            TokenStream::from_iter([
                TokenTree::Ident(Ident::new("doc", Span::call_site())),
                TokenTree::Punct(Punct::new('=', Spacing::Joint)),
                TokenTree::Literal(Literal::string(content)),
            ]),
        )),
    ]
}

fn extract_string(
    ts: &mut std::iter::Peekable<proc_macro::token_stream::IntoIter>,
) -> Result<(Literal, TokenStream), CompileError> {
    let next = ts.next();

    match next {
        Some(TokenTree::Literal(string)) => {
            // Success.
            Ok((string, TokenStream::new()))
        }
        Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis => {
            let mut stream = group.stream().into_iter();

            match stream.next() {
                Some(TokenTree::Literal(string)) => Ok((string, stream.collect())),
                Some(tt) => Err(CompileError::new(tt.span(), "expected string literal")),
                None => Err(CompileError::new(group.span(), "expected string literal")),
            }
        }
        Some(tt) => Err(CompileError::new(tt.span(), "expected string literal")),
        None => unreachable!("`=` is always followed by an expression"),
    }
}

/// A single arm like:
///
/// ```ignore
/// Self::InvalidHeader { expected, found, } => f.write_fmt(format_args!("..."))
/// ```
type DisplayArm = [TokenTree; 12];

/// Generates an arm like this:
///
/// ```ignore
/// Self::InvalidHeader { expected, found, } => f.write_fmt(format_args!("...", a, b, ))
///       ^^^^^^^^^^^^^ variant_ident
///                     ^^^^^^^^^^^^^^^^^^^^ destructure
///                                                                      ^^^^^ string
///                                                                           ^^^^^^^^ stream
/// ```
fn generate_arm(
    variant: &str,
    destructure: TokenTree,
    string: Literal,
    stream: TokenStream,
) -> DisplayArm {
    [
        TokenTree::Ident(Ident::new("Self", Span::call_site())),
        TokenTree::Punct(Punct::new(':', Spacing::Joint)),
        TokenTree::Punct(Punct::new(':', Spacing::Joint)),
        TokenTree::Ident(Ident::new(variant, Span::call_site())),
        destructure,
        TokenTree::Punct(Punct::new('=', Spacing::Joint)),
        TokenTree::Punct(Punct::new('>', Spacing::Joint)),
        TokenTree::Ident(Ident::new("f", Span::call_site())),
        TokenTree::Punct(Punct::new('.', Spacing::Joint)),
        TokenTree::Ident(Ident::new("write_fmt", Span::call_site())),
        TokenTree::Group(Group::new(
            Delimiter::Parenthesis,
            TokenStream::from_iter([
                TokenTree::Punct(Punct::new(':', Spacing::Joint)),
                TokenTree::Punct(Punct::new(':', Spacing::Joint)),
                TokenTree::Ident(Ident::new("core", Span::call_site())),
                TokenTree::Punct(Punct::new(':', Spacing::Joint)),
                TokenTree::Punct(Punct::new(':', Spacing::Joint)),
                TokenTree::Ident(Ident::new("format_args", Span::call_site())),
                TokenTree::Punct(Punct::new('!', Spacing::Joint)),
                TokenTree::Group(Group::new(
                    Delimiter::Parenthesis,
                    [TokenTree::Literal(string)]
                        .into_iter()
                        .chain(stream)
                        .collect(),
                )),
            ]),
        )),
        TokenTree::Punct(Punct::new(',', Spacing::Joint)),
    ]
}

/// `.into_iter()` generates `compile_error!($message)` at `$span`
struct CompileError {
    /// Where the compile error is generates
    pub span: Span,
    /// Message of the compile error
    pub message: String,
}

impl CompileError {
    /// Create a new compile error
    pub fn new(span: Span, message: impl AsRef<str>) -> Self {
        Self {
            span,
            message: message.as_ref().to_string(),
        }
    }
}

impl IntoIterator for CompileError {
    type Item = TokenTree;
    type IntoIter = std::array::IntoIter<Self::Item, 3>;

    fn into_iter(self) -> Self::IntoIter {
        [
            TokenTree::Ident(Ident::new("compile_error", self.span)),
            TokenTree::Punct({
                let mut punct = Punct::new('!', Spacing::Alone);
                punct.set_span(self.span);
                punct
            }),
            TokenTree::Group({
                let mut group = Group::new(Delimiter::Brace, {
                    TokenStream::from_iter(vec![TokenTree::Literal({
                        let mut string = Literal::string(&self.message);
                        string.set_span(self.span);
                        string
                    })])
                });
                group.set_span(self.span);
                group
            }),
        ]
        .into_iter()
    }
}
