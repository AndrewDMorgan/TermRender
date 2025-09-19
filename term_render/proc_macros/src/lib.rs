use proc_macro::TokenStream;
use quote::quote;

use syn::{parse_macro_input, Expr, Token};
use syn::punctuated::Punctuated;

/// Expands to a call to the Colorize trait. Colorize is implemented by default for...
/// * Colored
/// * String
/// * str
/// # Parameters
/// - value of type T with trait Colorize
/// - n ColorType variants (n: 0 - ∞)
/// # Example
/// ```
/// term_render::color!("Hello World", White, Bold, Underline);
/// term_render::color!("Hello World");  // converts to Colored without applying modifiers
/// ```
#[proc_macro]
pub fn color (input: TokenStream) -> TokenStream {
    let args: Punctuated<Expr, Token![,]> = parse_macro_input!(input with Punctuated::parse_terminated);
    let string = &args[0];
    let variants: Vec <_> = args.iter().skip(1).map(|ident| quote! { ColorType::#ident }).collect();

    if variants.len() == 1 {
        let arg = &args[1];
        return TokenStream::from (quote! {
            #string.colorize(ColorType::#arg)
        })
    }
    TokenStream::from (quote! {
        #string.colorizes(vec![#(#variants),*])
    })
}

