use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input, spanned::Spanned};

#[proc_macro_derive(SerializeWithDatabase)]
pub fn serialize_with_database(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let tokens = match &input.data {
        Data::Struct(syn::DataStruct { fields: syn::Fields::Named(fields), .. }) => {
             serialize_named_fields(&input, &fields.named)
        }
        _ => syn::Error::new(input.span(), "SerializeWithDatabase can only be used on structs with named fields. Did you put the #[derive(SerializeWithDatabase)] above #[salsa::tracked]?")
            .into_compile_error(),
    };

    tokens.into()
}

fn serialize_named_fields(
    input: &DeriveInput,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> proc_macro2::TokenStream {
    let (parent_impl_generics, parent_ty_generics, parent_where_clause) =
        input.generics.split_for_impl();

    let ident = &input.ident;
    let name = ident.to_string();

    let serialize_fields = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let as_string = field_name.to_string();
        quote! {
            {
                let value = self.inner.#field_name(self.db);
                let helper = crate::debug::helper(&value);
                let ser = helper.serialize_with_db(self.db);
                state.serialize_field(#as_string, &ser)?;
            }
        }
    });

    let mut child_generics = input.generics.clone();
    let lifetime: syn::GenericParam = syn::parse_quote!('_ref);
    child_generics.params.push(lifetime.clone());
    let (child_impl_generics, child_ty_generics, child_where_clause) =
        child_generics.split_for_impl();

    quote! {
        const _: () = {
            use serde::ser::SerializeStruct;
            use crate::debug::SerializeWithDatabase;

            impl #parent_impl_generics SerializeWithDatabase for #ident #parent_ty_generics #parent_where_clause {
                fn serialize_with_db<'a>(&'a self, db: &'a dyn salsa::Database) -> impl serde::Serialize + 'a {
                    struct Impl #child_ty_generics #child_where_clause {
                        db: & #lifetime dyn salsa::Database,
                        inner: & #lifetime #ident #parent_ty_generics,
                    }
                    impl #child_impl_generics serde::Serialize for Impl #child_ty_generics #child_where_clause {
                        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                        where
                            S: serde::Serializer,
                        {
                            let mut state = serializer.serialize_struct(#name, 2)?;
                            #(#serialize_fields)*
                            state.end()
                        }
                    }
                }
            }
        };
    }
}
