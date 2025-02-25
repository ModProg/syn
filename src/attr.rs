use super::*;
use proc_macro2::TokenStream;
use std::iter;
use std::slice;

#[cfg(feature = "parsing")]
use crate::meta::{self, ParseNestedMeta};
#[cfg(feature = "parsing")]
use crate::parse::{Parse, ParseStream, Parser, Result};
#[cfg(feature = "parsing")]
use std::fmt::Write;

ast_struct! {
    /// An attribute, like `#[repr(transparent)]`.
    ///
    /// <br>
    ///
    /// # Syntax
    ///
    /// Rust has six types of attributes.
    ///
    /// - Outer attributes like `#[repr(transparent)]`. These appear outside or
    ///   in front of the item they describe.
    ///
    /// - Inner attributes like `#![feature(proc_macro)]`. These appear inside
    ///   of the item they describe, usually a module.
    ///
    /// - Outer one-line doc comments like `/// Example`.
    ///
    /// - Inner one-line doc comments like `//! Please file an issue`.
    ///
    /// - Outer documentation blocks `/** Example */`.
    ///
    /// - Inner documentation blocks `/*! Please file an issue */`.
    ///
    /// The `style` field of type `AttrStyle` distinguishes whether an attribute
    /// is outer or inner.
    ///
    /// Every attribute has a `path` that indicates the intended interpretation
    /// of the rest of the attribute's contents. The path and the optional
    /// additional contents are represented together in the `meta` field of the
    /// attribute in three possible varieties:
    ///
    /// - Meta::Path &mdash; attributes whose information content conveys just a
    ///   path, for example the `#[test]` attribute.
    ///
    /// - Meta::List &mdash; attributes that carry arbitrary tokens after the
    ///   path, surrounded by a delimiter (parenthesis, bracket, or brace). For
    ///   example `#[derive(Copy)]` or `#[precondition(x < 5)]`.
    ///
    /// - Meta::NameValue &mdash; attributes with an `=` sign after the path,
    ///   followed by a Rust expression. For example `#[path =
    ///   "sys/windows.rs"]`.
    ///
    /// All doc comments are represented in the NameValue style with a path of
    /// "doc", as this is how they are processed by the compiler and by
    /// `macro_rules!` macros.
    ///
    /// ```text
    /// #[derive(Copy, Clone)]
    ///   ~~~~~~Path
    ///   ^^^^^^^^^^^^^^^^^^^Meta::List
    ///
    /// #[path = "sys/windows.rs"]
    ///   ~~~~Path
    ///   ^^^^^^^^^^^^^^^^^^^^^^^Meta::NameValue
    ///
    /// #[test]
    ///   ^^^^Meta::Path
    /// ```
    ///
    /// <br>
    ///
    /// # Parsing from tokens to Attribute
    ///
    /// This type does not implement the [`Parse`] trait and thus cannot be
    /// parsed directly by [`ParseStream::parse`]. Instead use
    /// [`ParseStream::call`] with one of the two parser functions
    /// [`Attribute::parse_outer`] or [`Attribute::parse_inner`] depending on
    /// which you intend to parse.
    ///
    /// [`Parse`]: parse::Parse
    /// [`ParseStream::parse`]: parse::ParseBuffer::parse
    /// [`ParseStream::call`]: parse::ParseBuffer::call
    ///
    /// ```
    /// use syn::{Attribute, Ident, Result, Token};
    /// use syn::parse::{Parse, ParseStream};
    ///
    /// // Parses a unit struct with attributes.
    /// //
    /// //     #[path = "s.tmpl"]
    /// //     struct S;
    /// struct UnitStruct {
    ///     attrs: Vec<Attribute>,
    ///     struct_token: Token![struct],
    ///     name: Ident,
    ///     semi_token: Token![;],
    /// }
    ///
    /// impl Parse for UnitStruct {
    ///     fn parse(input: ParseStream) -> Result<Self> {
    ///         Ok(UnitStruct {
    ///             attrs: input.call(Attribute::parse_outer)?,
    ///             struct_token: input.parse()?,
    ///             name: input.parse()?,
    ///             semi_token: input.parse()?,
    ///         })
    ///     }
    /// }
    /// ```
    ///
    /// <p><br></p>
    ///
    /// # Parsing from Attribute to structured arguments
    ///
    /// The grammar of attributes in Rust is very flexible, which makes the
    /// syntax tree not that useful on its own. In particular, arguments of the
    /// `Meta::List` variety of attribute are held in an arbitrary `tokens:
    /// TokenStream`. Macros are expected to check the `path` of the attribute,
    /// decide whether they recognize it, and then parse the remaining tokens
    /// according to whatever grammar they wish to require for that kind of
    /// attribute. Use [`parse_args()`] to parse those tokens into the expected
    /// data structure.
    ///
    /// [`parse_args()`]: Attribute::parse_args
    ///
    /// <p><br></p>
    ///
    /// # Doc comments
    ///
    /// The compiler transforms doc comments, such as `/// comment` and `/*!
    /// comment */`, into attributes before macros are expanded. Each comment is
    /// expanded into an attribute of the form `#[doc = r"comment"]`.
    ///
    /// As an example, the following `mod` items are expanded identically:
    ///
    /// ```
    /// # use syn::{ItemMod, parse_quote};
    /// let doc: ItemMod = parse_quote! {
    ///     /// Single line doc comments
    ///     /// We write so many!
    ///     /**
    ///      * Multi-line comments...
    ///      * May span many lines
    ///      */
    ///     mod example {
    ///         //! Of course, they can be inner too
    ///         /*! And fit in a single line */
    ///     }
    /// };
    /// let attr: ItemMod = parse_quote! {
    ///     #[doc = r" Single line doc comments"]
    ///     #[doc = r" We write so many!"]
    ///     #[doc = r"
    ///      * Multi-line comments...
    ///      * May span many lines
    ///      "]
    ///     mod example {
    ///         #![doc = r" Of course, they can be inner too"]
    ///         #![doc = r" And fit in a single line "]
    ///     }
    /// };
    /// assert_eq!(doc, attr);
    /// ```
    #[cfg_attr(doc_cfg, doc(cfg(any(feature = "full", feature = "derive"))))]
    pub struct Attribute {
        pub pound_token: Token![#],
        pub style: AttrStyle,
        pub bracket_token: token::Bracket,
        pub meta: Meta,
    }
}

impl Attribute {
    /// Returns the path that identifies the interpretation of this attribute.
    ///
    /// For example this would return the `test` in `#[test]`, the `derive` in
    /// `#[derive(Copy)]`, and the `path` in `#[path = "sys/windows.rs"]`.
    pub fn path(&self) -> &Path {
        self.meta.path()
    }

    /// Parse the arguments to the attribute as a syntax tree.
    ///
    /// This is similar to pulling out the `TokenStream` from `Meta::List` and
    /// doing `syn::parse2::<T>(meta_list.tokens)`, except that using
    /// `parse_args` the error message has a more useful span when `tokens` is
    /// empty.
    ///
    /// The surrounding delimiters are *not* included in the input to the
    /// parser.
    ///
    /// ```text
    /// #[my_attr(value < 5)]
    ///           ^^^^^^^^^ what gets parsed
    /// ```
    ///
    /// # Example
    ///
    /// ```
    /// # fn example() -> syn::Result<()> {
    /// use syn::{parse_quote, Attribute, Expr};
    ///
    /// let attr: Attribute = parse_quote! {
    ///     #[precondition(value < 5)]
    /// };
    ///
    /// if attr.path().is_ident("precondition") {
    ///     let precondition: Expr = attr.parse_args()?;
    ///     // ...
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "parsing")]
    #[cfg_attr(doc_cfg, doc(cfg(feature = "parsing")))]
    pub fn parse_args<T: Parse>(&self) -> Result<T> {
        self.parse_args_with(T::parse)
    }

    /// Parse the arguments to the attribute using the given parser.
    ///
    /// # Example
    ///
    /// ```
    /// # fn example() -> syn::Result<()> {
    /// use syn::{parse_quote, Attribute};
    ///
    /// let attr: Attribute = parse_quote! {
    ///     #[inception { #[brrrrrrraaaaawwwwrwrrrmrmrmmrmrmmmmm] }]
    /// };
    ///
    /// let bwom = attr.parse_args_with(Attribute::parse_outer)?;
    ///
    /// // Attribute does not have a Parse impl, so we couldn't directly do:
    /// // let bwom: Attribute = attr.parse_args()?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "parsing")]
    #[cfg_attr(doc_cfg, doc(cfg(feature = "parsing")))]
    pub fn parse_args_with<F: Parser>(&self, parser: F) -> Result<F::Output> {
        match &self.meta {
            Meta::Path(path) => {
                let expected = expected_parentheses(&self.style, path);
                let msg = format!("expected attribute arguments in parentheses: {}", expected);
                Err(crate::error::new2(
                    self.pound_token.span,
                    self.bracket_token.span,
                    msg,
                ))
            }
            Meta::NameValue(meta) => {
                let expected = expected_parentheses(&self.style, &meta.path);
                let msg = format!("expected parentheses: {}", expected);
                Err(Error::new(meta.eq_token.span, msg))
            }
            Meta::List(meta) => parser.parse2(meta.tokens.clone()),
        }
    }

    /// Parse the arguments to the attribute, expecting it to follow the
    /// conventional structure used by most of Rust's built-in attributes.
    ///
    /// The [*Meta Item Attribute Syntax*][syntax] section in the Rust reference
    /// explains the convention in more detail. Not all attributes follow this
    /// convention, so [`parse_args()`][Self::parse_args] is available if you
    /// need to parse arbitrarily goofy attribute syntax.
    ///
    /// [syntax]: https://doc.rust-lang.org/reference/attributes.html#meta-item-attribute-syntax
    ///
    /// # Example
    ///
    /// We'll parse a struct, and then parse some of Rust's `#[repr]` attribute
    /// syntax.
    ///
    /// ```
    /// # fn example() -> syn::Result<()> {
    /// use syn::{parenthesized, parse_quote, token, ItemStruct, LitInt};
    ///
    /// let input: ItemStruct = parse_quote! {
    ///     #[repr(C, align(4))]
    ///     pub struct MyStruct(u16, u32);
    /// };
    ///
    /// let mut repr_c = false;
    /// let mut repr_transparent = false;
    /// let mut repr_align = None::<usize>;
    /// let mut repr_packed = None::<usize>;
    /// for attr in &input.attrs {
    ///     if attr.path().is_ident("repr") {
    ///         attr.parse_nested_meta(|meta| {
    ///             if meta.path.is_ident("C") {
    ///                 // #[repr(C)]
    ///                 repr_c = true;
    ///             } else if meta.path.is_ident("transparent") {
    ///                 // #[repr(transparent)]
    ///                 repr_transparent = true;
    ///             } else if meta.path.is_ident("align") {
    ///                 // #[repr(align(N))]
    ///                 let content;
    ///                 parenthesized!(content in meta.input);
    ///                 let lit: LitInt = content.parse()?;
    ///                 let n: usize = lit.base10_parse()?;
    ///                 repr_align = Some(n);
    ///             } else if meta.path.is_ident("packed") {
    ///                 // #[repr(packed)] or #[repr(packed(N))], omitted N means 1
    ///                 if meta.input.peek(token::Paren) {
    ///                     let content;
    ///                     parenthesized!(content in meta.input);
    ///                     let lit: LitInt = content.parse()?;
    ///                     let n: usize = lit.base10_parse()?;
    ///                     repr_packed = Some(n);
    ///                 } else {
    ///                     repr_packed = Some(1);
    ///                 }
    ///             } else {
    ///                 return Err(meta.error("unrecognized repr"));
    ///             }
    ///             Ok(())
    ///         })?;
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "parsing")]
    #[cfg_attr(doc_cfg, doc(cfg(feature = "parsing")))]
    pub fn parse_nested_meta(
        &self,
        logic: impl FnMut(ParseNestedMeta) -> Result<()>,
    ) -> Result<()> {
        self.parse_args_with(meta::parser(logic))
    }

    /// Parses zero or more outer attributes from the stream.
    ///
    /// # Example
    ///
    /// See
    /// [*Parsing from tokens to Attribute*](#parsing-from-tokens-to-attribute).
    #[cfg(feature = "parsing")]
    #[cfg_attr(doc_cfg, doc(cfg(feature = "parsing")))]
    pub fn parse_outer(input: ParseStream) -> Result<Vec<Self>> {
        let mut attrs = Vec::new();
        while input.peek(Token![#]) {
            attrs.push(input.call(parsing::single_parse_outer)?);
        }
        Ok(attrs)
    }

    /// Parses zero or more inner attributes from the stream.
    ///
    /// # Example
    ///
    /// See
    /// [*Parsing from tokens to Attribute*](#parsing-from-tokens-to-attribute).
    #[cfg(feature = "parsing")]
    #[cfg_attr(doc_cfg, doc(cfg(feature = "parsing")))]
    pub fn parse_inner(input: ParseStream) -> Result<Vec<Self>> {
        let mut attrs = Vec::new();
        parsing::parse_inner(input, &mut attrs)?;
        Ok(attrs)
    }
}

#[cfg(feature = "parsing")]
fn expected_parentheses(style: &AttrStyle, path: &Path) -> String {
    let mut suggestion = String::new();
    match style {
        AttrStyle::Outer => suggestion.push('#'),
        AttrStyle::Inner(_) => suggestion.push_str("#!"),
    }
    suggestion.push('[');

    for (i, segment) in path.segments.iter().enumerate() {
        if i > 0 || path.leading_colon.is_some() {
            suggestion.push_str("::");
        }
        write!(suggestion, "{}", segment.ident).unwrap();
    }

    suggestion.push_str("(...)]");
    suggestion
}

ast_enum! {
    /// Distinguishes between attributes that decorate an item and attributes
    /// that are contained within an item.
    ///
    /// # Outer attributes
    ///
    /// - `#[repr(transparent)]`
    /// - `/// # Example`
    /// - `/** Please file an issue */`
    ///
    /// # Inner attributes
    ///
    /// - `#![feature(proc_macro)]`
    /// - `//! # Example`
    /// - `/*! Please file an issue */`
    #[cfg_attr(doc_cfg, doc(cfg(any(feature = "full", feature = "derive"))))]
    pub enum AttrStyle {
        Outer,
        Inner(Token![!]),
    }
}

ast_enum_of_structs! {
    /// Content of a compile-time structured attribute.
    ///
    /// ## Path
    ///
    /// A meta path is like the `test` in `#[test]`.
    ///
    /// ## List
    ///
    /// A meta list is like the `derive(Copy)` in `#[derive(Copy)]`.
    ///
    /// ## NameValue
    ///
    /// A name-value meta is like the `path = "..."` in `#[path =
    /// "sys/windows.rs"]`.
    ///
    /// # Syntax tree enum
    ///
    /// This type is a [syntax tree enum].
    ///
    /// [syntax tree enum]: Expr#syntax-tree-enums
    #[cfg_attr(doc_cfg, doc(cfg(any(feature = "full", feature = "derive"))))]
    pub enum Meta {
        Path(Path),

        /// A structured list within an attribute, like `derive(Copy, Clone)`.
        List(MetaList),

        /// A name-value pair within an attribute, like `feature = "nightly"`.
        NameValue(MetaNameValue),
    }
}

ast_struct! {
    /// A structured list within an attribute, like `derive(Copy, Clone)`.
    #[cfg_attr(doc_cfg, doc(cfg(any(feature = "full", feature = "derive"))))]
    pub struct MetaList {
        pub path: Path,
        pub delimiter: MacroDelimiter,
        pub tokens: TokenStream,
    }
}

ast_struct! {
    /// A name-value pair within an attribute, like `feature = "nightly"`.
    #[cfg_attr(doc_cfg, doc(cfg(any(feature = "full", feature = "derive"))))]
    pub struct MetaNameValue {
        pub path: Path,
        pub eq_token: Token![=],
        pub value: Expr,
    }
}

impl Meta {
    /// Returns the path that begins this structured meta item.
    ///
    /// For example this would return the `test` in `#[test]`, the `derive` in
    /// `#[derive(Copy)]`, and the `path` in `#[path = "sys/windows.rs"]`.
    pub fn path(&self) -> &Path {
        match self {
            Meta::Path(path) => path,
            Meta::List(meta) => &meta.path,
            Meta::NameValue(meta) => &meta.path,
        }
    }
}

pub(crate) trait FilterAttrs<'a> {
    type Ret: Iterator<Item = &'a Attribute>;

    fn outer(self) -> Self::Ret;
    fn inner(self) -> Self::Ret;
}

impl<'a> FilterAttrs<'a> for &'a [Attribute] {
    type Ret = iter::Filter<slice::Iter<'a, Attribute>, fn(&&Attribute) -> bool>;

    fn outer(self) -> Self::Ret {
        fn is_outer(attr: &&Attribute) -> bool {
            match attr.style {
                AttrStyle::Outer => true,
                AttrStyle::Inner(_) => false,
            }
        }
        self.iter().filter(is_outer)
    }

    fn inner(self) -> Self::Ret {
        fn is_inner(attr: &&Attribute) -> bool {
            match attr.style {
                AttrStyle::Inner(_) => true,
                AttrStyle::Outer => false,
            }
        }
        self.iter().filter(is_inner)
    }
}

#[cfg(feature = "parsing")]
pub(crate) mod parsing {
    use super::*;
    use crate::ext::IdentExt;
    use crate::parse::{Parse, ParseStream, Result};

    pub(crate) fn parse_inner(input: ParseStream, attrs: &mut Vec<Attribute>) -> Result<()> {
        while input.peek(Token![#]) && input.peek2(Token![!]) {
            attrs.push(input.call(parsing::single_parse_inner)?);
        }
        Ok(())
    }

    pub(crate) fn single_parse_inner(input: ParseStream) -> Result<Attribute> {
        let content;
        Ok(Attribute {
            pound_token: input.parse()?,
            style: AttrStyle::Inner(input.parse()?),
            bracket_token: bracketed!(content in input),
            meta: content.parse()?,
        })
    }

    pub(crate) fn single_parse_outer(input: ParseStream) -> Result<Attribute> {
        let content;
        Ok(Attribute {
            pound_token: input.parse()?,
            style: AttrStyle::Outer,
            bracket_token: bracketed!(content in input),
            meta: content.parse()?,
        })
    }

    #[cfg_attr(doc_cfg, doc(cfg(feature = "parsing")))]
    impl Parse for Meta {
        fn parse(input: ParseStream) -> Result<Self> {
            if cfg!(feature = "full") && input.peek(Token![unsafe]) {
                let unsafe_ident = Ident::parse_any(input)?;
                parse_meta_list_after_path(Path::from(unsafe_ident), input).map(Meta::List)
            } else {
                let path = input.call(Path::parse_mod_style)?;
                parse_meta_after_path(path, input)
            }
        }
    }

    #[cfg_attr(doc_cfg, doc(cfg(feature = "parsing")))]
    impl Parse for MetaList {
        fn parse(input: ParseStream) -> Result<Self> {
            let path = input.call(Path::parse_mod_style)?;
            parse_meta_list_after_path(path, input)
        }
    }

    #[cfg_attr(doc_cfg, doc(cfg(feature = "parsing")))]
    impl Parse for MetaNameValue {
        fn parse(input: ParseStream) -> Result<Self> {
            let path = input.call(Path::parse_mod_style)?;
            parse_meta_name_value_after_path(path, input)
        }
    }

    pub(crate) fn parse_meta_after_path(path: Path, input: ParseStream) -> Result<Meta> {
        if input.peek(token::Paren) || input.peek(token::Bracket) || input.peek(token::Brace) {
            parse_meta_list_after_path(path, input).map(Meta::List)
        } else if input.peek(Token![=]) {
            parse_meta_name_value_after_path(path, input).map(Meta::NameValue)
        } else {
            Ok(Meta::Path(path))
        }
    }

    fn parse_meta_list_after_path(path: Path, input: ParseStream) -> Result<MetaList> {
        let (delimiter, tokens) = mac::parse_delimiter(input)?;
        Ok(MetaList {
            path,
            delimiter,
            tokens,
        })
    }

    fn parse_meta_name_value_after_path(path: Path, input: ParseStream) -> Result<MetaNameValue> {
        Ok(MetaNameValue {
            path,
            eq_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

#[cfg(feature = "printing")]
mod printing {
    use super::*;
    use proc_macro2::TokenStream;
    use quote::ToTokens;

    #[cfg_attr(doc_cfg, doc(cfg(feature = "printing")))]
    impl ToTokens for Attribute {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.pound_token.to_tokens(tokens);
            if let AttrStyle::Inner(b) = &self.style {
                b.to_tokens(tokens);
            }
            self.bracket_token.surround(tokens, |tokens| {
                self.meta.to_tokens(tokens);
            });
        }
    }

    #[cfg_attr(doc_cfg, doc(cfg(feature = "printing")))]
    impl ToTokens for MetaList {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.path.to_tokens(tokens);
            self.delimiter.surround(tokens, self.tokens.clone());
        }
    }

    #[cfg_attr(doc_cfg, doc(cfg(feature = "printing")))]
    impl ToTokens for MetaNameValue {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.path.to_tokens(tokens);
            self.eq_token.to_tokens(tokens);
            self.value.to_tokens(tokens);
        }
    }
}
