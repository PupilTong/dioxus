mod css;
mod inline_image;

use inline_image::InlineImageInput;
use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro]
#[allow(non_snake_case)]
pub fn CSS(input: TokenStream) -> TokenStream {
    css::css(input)
}

#[proc_macro]
pub fn inline_image(input: TokenStream) -> TokenStream {
    let image = parse_macro_input!(input as InlineImageInput);
    TokenStream::from(image.into_token_stream())
}
