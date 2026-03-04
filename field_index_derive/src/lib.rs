use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Fields, Ident, Type, parse_macro_input};

#[proc_macro_derive(IndexField, attributes(field))]
pub fn derive_index_field(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    let enum_name = Ident::new(&format!("Indexable{}", struct_name), Span::call_site());

    let fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(f) => &f.named,
            _ => panic!("IndexField: only named fields are supported"),
        },
        _ => panic!("IndexField: only structs are supported"),
    };

    let mut variant_defs = vec![];
    let mut parse_arms = vec![];
    let mut apply_arms = vec![];
    let mut get_arms = vec![];
    let mut set_arms = vec![];
    let mut display_arms = vec![];

    for field in fields.iter() {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let variant_name = Ident::new(&snake_to_pascal(&field_name.to_string()), Span::call_site());
        let field_name_str = field_name.to_string();

        if is_nested(&field.attrs) {
            let nested_enum = nested_name(field_type);

            variant_defs.push(quote! {
                #variant_name(#nested_enum),
            });

            parse_arms.push(quote! {
                #field_name_str => {
                    let rest = parts.next().ok_or_else(|| format!("expected subfield after '{}'", #field_name_str))?;
                    Ok(#enum_name::#variant_name(#nested_enum::parse(rest, value)?))
                }
            });

            apply_arms.push(quote! {
                #enum_name::#variant_name(f) => self.#field_name.apply_field(f),
            });

            get_arms.push(quote! {
                            #field_name_str => serde_json::to_value(&self.#field_name)
            .map_err(|e| format!("field '{}': {}", #field_name_str, e))
                        });

            display_arms.push(quote! {
                #enum_name::#variant_name(v) => write!(f, "{}", v)
            });

            set_arms.push(quote! {
                #field_name_str => {
        let rest = parts.next()
            .ok_or_else(|| format!("'{}' is a nested field, specify a subfield", #field_name_str))?;
        self.#field_name.set_field(rest, value)
    }
            });
        } else {
            variant_defs.push(quote! {
                #variant_name(#field_type)
            });

            // Use FromStr to parse the value string into the field's type
            if let Some(inner_ty) = extract_option_inner(field_type) {
                parse_arms.push(quote! {
                    #field_name_str => {
                        if value.is_empty() || value.eq_ignore_ascii_case("none") {
                            return Ok(#enum_name::#variant_name(None));
                        }
                        let parsed = value.parse::<#inner_ty>()
                            .map_err(|e| format!(
                                "field '{}' expects {}, got {:?}: {}",
                                #field_name_str, stringify!(#inner_ty), value, e.to_string()
                            ))?;
                        Ok(#enum_name::#variant_name(Some(parsed)))
                    }
                });

                display_arms.push(quote! {
                    #enum_name::#variant_name(v) => match v {
                        Some(v) => write!(f, "{}", v),
                        None => write!(f, "none"),
                    }
                });

                get_arms.push(quote! {
                                #field_name_str => serde_json::to_value(&self.#field_name)
                .map_err(|e| format!("field '{}': {}", #field_name_str, e))
                            });

                set_arms.push(quote! {
                    #field_name_str => {
        self.#field_name = serde_json::from_value::<std::option::Option<#inner_ty>>(value)
            .map_err(|e| format!("field '{}': {}", #field_name_str, e))?;
        Ok(())
    }
                });
            } else {
                parse_arms.push(quote! {
                    #field_name_str => {
                        let parsed = value.parse::<#field_type>()
                            .map_err(|e| format!(
                                "field '{}' expects {}, got {:?}: {}",
                                #field_name_str, stringify!(#field_type), value, e.to_string()
                            ))?;
                        Ok(#enum_name::#variant_name(parsed))
                    }
                });
                get_arms.push(quote! {
                                #field_name_str => serde_json::to_value(&self.#field_name)
                .map_err(|e| format!("field '{}': {}", #field_name_str, e))
                            });

                display_arms.push(quote! {
                    #enum_name::#variant_name(v) => write!(f, "{}", v)
                });

                set_arms.push(quote! {
                   #field_name_str => {
                        self.#field_name = serde_json::from_value::<#field_type>(value)
                            .map_err(|e| format!("field '{}': {}", #field_name_str, e))?;
                        Ok(())
                    }
                });
            }

            apply_arms.push(quote! {
                #enum_name::#variant_name(v) => self.#field_name = v,
            });
        }
    }

    quote! {
           #[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
           pub enum #enum_name {
               #(#variant_defs,)*
           }

        impl std::fmt::Display for #enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                match self {
                    #(#display_arms,)*
                }
            }
        }

           impl index_field::IndexField for #struct_name {
    fn get_field(&self, path: &str) -> std::result::Result<serde_json::Value, String> {
               let mut parts = path.splitn(2, '.');
               match parts.next().unwrap() {
                   #(#get_arms,)*
                   other => Err(format!("unknown field {:?}", other)),
               }
           }

           fn set_field(&mut self, path: &str, value: serde_json::Value) -> std::result::Result<(), String> {
               let mut parts = path.splitn(2, '.');
               match parts.next().unwrap() {
                   #(#set_arms,)*  // reuse existing parse arms but apply instead of return
                   other => Err(format!("unknown field {:?}", other)),
               }
           }
           }

           impl #struct_name {
               pub fn apply_field(&mut self, field: #enum_name) {
                   match field {
                       #(#apply_arms)*
                   }
               }
           }
       }
    .into()
}

fn extract_option_inner(ty: &Type) -> Option<&Type> {
    let path = match ty {
        Type::Path(p) => &p.path,
        _ => return None,
    };
    let last = path.segments.last()?;
    // match both `Option` and `std::option::Option`
    if last.ident != "Option" {
        return None;
    }
    match &last.arguments {
        syn::PathArguments::AngleBracketed(args) => match args.args.first()? {
            syn::GenericArgument::Type(t) => Some(t),
            _ => None,
        },
        _ => None,
    }
}

fn is_nested(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("field") {
            return false;
        }
        let mut nested = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("nested") {
                nested = true;
            }
            Ok(())
        });
        nested
    })
}

fn nested_name(t: &Type) -> Ident {
    match t {
        Type::Path(p) => {
            let last = p.path.segments.last().unwrap();
            Ident::new(&format!("{}Fields", last.ident), Span::call_site())
        }
        _ => panic!("Nested path type not supported"),
    }
}

fn snake_to_pascal(s: &str) -> String {
    s.split('_')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect()
}
