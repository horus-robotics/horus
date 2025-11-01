//! Message macro implementation
//!
//! Provides the `message!` macro for easy message type definitions.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Comma,
    Field, Ident, Result, Token, Type,
};

/// Parse either tuple-style or struct-style message definition
pub enum MessageInput {
    /// Tuple-style: `Position = (f32, f32)`
    Tuple {
        name: Ident,
        types: Vec<Type>,
    },
    /// Struct-style: `MyMessage { x: u8, y: u8 }`
    Struct {
        name: Ident,
        fields: Vec<(Ident, Type)>,
    },
}

impl Parse for MessageInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;

        // Check if it's tuple-style (with =) or struct-style (with {)
        if input.peek(Token![=]) {
            // Tuple-style: Position = (f32, f32)
            input.parse::<Token![=]>()?;

            let content;
            syn::parenthesized!(content in input);

            let types: Punctuated<Type, Comma> = content.parse_terminated(Type::parse, Token![,])?;
            let types: Vec<Type> = types.into_iter().collect();

            Ok(MessageInput::Tuple { name, types })
        } else {
            // Struct-style: MyMessage { x: u8, y: u8 }
            let content;
            syn::braced!(content in input);

            let fields: Punctuated<Field, Comma> = content.parse_terminated(Field::parse_named, Token![,])?;

            let fields: Vec<(Ident, Type)> = fields
                .into_iter()
                .map(|f| (f.ident.unwrap(), f.ty))
                .collect();

            Ok(MessageInput::Struct { name, fields })
        }
    }
}

/// Generate the complete message implementation
pub fn generate_message(input: MessageInput) -> TokenStream {
    match input {
        MessageInput::Tuple { name, types } => generate_tuple_message(name, types),
        MessageInput::Struct { name, fields } => generate_struct_message(name, fields),
    }
}

/// Generate a tuple-style message
fn generate_tuple_message(name: Ident, types: Vec<Type>) -> TokenStream {
    let field_list = types.iter().map(|ty| {
        quote! { pub #ty }
    });

    quote! {
        #[derive(Debug, Clone, ::serde::Serialize, ::serde::Deserialize)]
        #[repr(C)]
        pub struct #name(#(#field_list),*);

        // Auto-implement LogSummary using Debug
        impl ::horus::core::LogSummary for #name {
            fn log_summary(&self) -> ::std::string::String {
                format!("{:?}", self)
            }
        }
    }
}

/// Generate a struct-style message with named fields
fn generate_struct_message(name: Ident, fields: Vec<(Ident, Type)>) -> TokenStream {
    let field_defs = fields.iter().map(|(field_name, field_type)| {
        quote! { pub #field_name: #field_type }
    });

    quote! {
        #[derive(Debug, Clone, ::serde::Serialize, ::serde::Deserialize)]
        #[repr(C)]
        pub struct #name {
            #(#field_defs),*
        }

        // Auto-implement LogSummary using Debug
        // Uses the LogSummary trait that should be in scope from horus::prelude::*
        impl ::horus::core::LogSummary for #name {
            fn log_summary(&self) -> ::std::string::String {
                format!("{:?}", self)
            }
        }

        // Enable zero-copy traits if all fields are Pod
        // Note: This will only compile if all fields actually implement Pod
        // If they don't, users can still use the type, just without zero-copy optimization
    }
}
