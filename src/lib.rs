use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{
    parse_macro_input, Data, DeriveInput, Ident, Token, Visibility,
};

#[proc_macro_derive(EnumComponentTag, attributes(require, tag_visibility))]
pub fn derive_enum_component_tag(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = &input.ident;
    let vis = &input.vis;

    // Extract tag visibility from attributes, defaulting to public
    let mut tag_visibility = Visibility::Public(Token![pub](Span::call_site()));
    for attr in &input.attrs {
        if attr.path().is_ident("tag_visibility") {
            let mut visibility = None;
            let _ = attr.parse_nested_meta(|meta| {
                if let Ok(value) = meta.value() {
                    visibility = Some(value.parse()?);
                }
                Ok(())
            });
            if let Some(vis) = visibility {
                tag_visibility = vis;
            }
        }
    }

    // Ensure the input is an enum
    let Data::Enum(ref data) = input.data else {
        return syn::Error::new(input.span(), "Cannot derive `EnumComponentTag` on non-enum type")
            .into_compile_error()
            .into();
    };

    // Process variants and their attributes
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
                            })
                            .unwrap_or_default(),
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

    // Generate module name based on enum name
    let mod_ident = format_ident!("{}", ident.to_string().to_case(Case::Snake));

    // Extract variant idents and their required components
    let variant_idents: Vec<_> = variants_with_attrs.iter().map(|(ident, _)| ident).collect();
    let require_idents: Vec<_> = variants_with_attrs.iter().map(|(_, list)| list).collect();

    // Generate the expanded code
    let expanded = quote! {
        impl bevy::ecs::component::Component for #ident {
            const STORAGE_TYPE: bevy::ecs::component::StorageType = bevy::ecs::component::StorageType::Table;
            fn register_component_hooks(hooks: &mut bevy::ecs::component::ComponentHooks) {
                hooks.on_add(#ident::enter_hook);
                hooks.on_insert(#ident::enter_hook);
                hooks.on_remove(#ident::exit_hook);
            }
        }

        impl #ident {
            fn enter_hook(mut world: bevy::ecs::world::DeferredWorld,
                          entity: bevy::ecs::entity::Entity,
                          _id: bevy::ecs::component::ComponentId) {
                #(
                    // Remove previously inserted tags, if present
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