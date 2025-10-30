use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_attribute]
pub fn strategy(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;

    const PLUGIN_CREATE_FUNCTION_NAME: &'static str = "_plugin_create";
    let func_name = syn::Ident::new(PLUGIN_CREATE_FUNCTION_NAME, name.span());

    let expanded = quote! {
        #input

        #[unsafe(no_mangle)]
        pub fn #func_name() -> *mut dyn ::merco::Strategy {
            let strategy = <#name as ::std::default::Default>::default();
            Box::into_raw(Box::new(strategy))
        }
    };

    TokenStream::from(expanded)
}
