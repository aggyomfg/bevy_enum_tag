use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{
    parse_macro_input, Data, DeriveInput, Ident, Token, Visibility,
};

#[proc_macro_attribute]
pub fn derive_enum_tag(attr: TokenStream, input: TokenStream) -> TokenStream {
    let mut tag_visibility = Visibility::Public(Token![pub](Span::call_site()));

    let visibility_parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("tag_visibility") {
            Ok(tag_visibility = Some(meta.value()?.parse()?).unwrap())
        } else {
            return Err(meta.error(format!(
                "Unsupported argument: {}",
                meta.path.get_ident().unwrap()
            )));
        }
    });

    parse_macro_input!(attr with visibility_parser);

    let input = parse_macro_input!(input as DeriveInput);
    let ident = &input.ident;
    let vis = &input.vis;

    let Data::Enum(ref data) = input.data else {
        return syn::Error::new(input.span(), "Cannot derive `EnumTrait` on non-enum type")
            .into_compile_error()
            .into();
    };

    let variants_with_attrs = data.variants.iter().map(|variant| {
        let ident = &variant.ident;
        let require_attrs = variant
            .attrs
            .iter()
            .filter_map(|attr| {
                if attr.path().is_ident("require") {
                    match attr.meta.require_list() {
                        Ok(list) => Some(
                            list.parse_args_with(|input: syn::parse::ParseStream| {
                                let mut idents = Vec::new();
                                while !input.is_empty() {
                                    let path: syn::Path = input.parse()?;
                                    if let Some(ident) = path.get_ident() {
                                        idents.push(ident.clone());
                                    }
                                    if !input.is_empty() {
                                        input.parse::<Token![,]>()?;
                                    }
                                }
                                Ok(idents)
                            }).unwrap_or_default(),
                        ),
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .flatten()
            .collect::<Vec<_>>();
        (ident.clone(), require_attrs)
    }).collect::<Vec<(Ident, Vec<Ident>)>>();

    let mod_ident = format_ident!("{}", ident.to_string().to_case(Case::Snake));
    let variant_idents: Vec<_> = variants_with_attrs.iter().map(|(ident, _)| ident).collect();
    let require_idents: Vec<_> = variants_with_attrs.iter().map(|(_, list)| list).collect();

    let expanded = quote! {
        #[derive(bevy::prelude::Component)]
        #[component(on_add = #ident::enter_hook)]
        #[component(on_insert = #ident::enter_hook)]
        #[component(on_remove = #ident::exit_hook)]
        #input

        impl #ident {
            fn enter_hook(mut world: bevy::ecs::world::DeferredWorld,
                          entity: bevy::ecs::entity::Entity,
                          _id: bevy::ecs::component::ComponentId) {
                #(
                    // remove previously inserted tags, if present
                    if world.entity(entity).get::<#mod_ident::#variant_idents>().is_some() {
                        world.commands().entity(entity).remove::<#mod_ident::#variant_idents>();
                    }
                )*
                match world.entity(entity).get::<#ident>() {
                    Some(enum_ref) => match enum_ref {
                        #(
                            #ident::#variant_idents { .. } => {
                                world.commands().entity(entity).insert(#mod_ident::#variant_idents);
                            }
                        )*
                    },
                    None => {}
                }
            }

            fn exit_hook(mut world: bevy::ecs::world::DeferredWorld,
                         entity: bevy::ecs::entity::Entity,
                         _id: bevy::ecs::component::ComponentId) {
                match world.entity(entity).get::<#ident>() {
                    Some(enum_ref) => match enum_ref {
                        #(
                            #ident::#variant_idents { .. } => {
                                world.commands().entity(entity).remove::<#mod_ident::#variant_idents>();
                            }
                        )*
                    },
                    None => {}
                }
            }
        }

        #vis mod #mod_ident {
            use super::*;

            #(
                #[derive(bevy::prelude::Component)]
                #[component(on_add = #variant_idents::enter_hook)]
                #[component(on_insert = #variant_idents::enter_hook)]
                #[require(#(#require_idents),*)]
                #tag_visibility struct #variant_idents;

                impl #variant_idents {
                    fn enter_hook(mut world: bevy::ecs::world::DeferredWorld,
                                  entity: bevy::ecs::entity::Entity,
                                  id: bevy::ecs::component::ComponentId) {
                        if let Some(#ident::#variant_idents {..}) = world.entity(entity).get::<#ident>() {
                        } else {
                            world.commands().entity(entity).remove_by_id(id);
                        }
                    }
                }
            )*
        }
    };

    TokenStream::from(expanded)
}