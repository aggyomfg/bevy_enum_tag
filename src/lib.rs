use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro_crate::{crate_name, FoundCrate};
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{parse_macro_input, Data, DeriveInput};

#[proc_macro_attribute]
pub fn derive_enum_tag(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = &input.ident;
    let vis = &input.vis;
    let bevy = get_crate("bevy");

    let Data::Enum(ref data) = input.data else {
        return syn::Error::new(
            input.span(),
            "Cannot derive `EnumTrait` on non-enum type")
            .into_compile_error().into()
    };

    let variants = &data.variants;

    let variant_idents = data
        .variants
        .iter()
        .map(|variant| { &variant.ident })
        .collect::<Vec<_>>();

    let mod_ident = format_ident!("{}", ident.to_string().to_case(Case::Snake));

    TokenStream::from(quote! {
        #[derive(bevy::prelude::Component)]
        #[component(on_add = #ident::enter_hook)]
        #[component(on_insert = #ident::enter_hook)]
        #[component(on_remove = #ident::exit_hook)]
        #vis enum #ident {
            #variants
        }
        
        impl #ident {
            fn enter_hook(mut world: bevy::ecs::world::DeferredWorld, 
                    entity: bevy::ecs::entity::Entity, 
                    _id: bevy::ecs::component::ComponentId) {
                match world.entity(entity).components::<&#ident>() {
                    #(
                        #ident::#variant_idents { .. } => {
                            world.commands().entity(entity).insert(#mod_ident::#variant_idents);
                        }
                    )*
                    _ => {}
                }
            }
            fn exit_hook(mut world: bevy::ecs::world::DeferredWorld, 
                    entity: bevy::ecs::entity::Entity, 
                    _id: bevy::ecs::component::ComponentId) {
                match world.entity(entity).components::<&#ident>() {
                    #(
                        #ident::#variant_idents { .. } => {
                            world.commands().entity(entity).remove::<#mod_ident::#variant_idents>();
                        }
                    )*
                    _ => {}
                }
            }
        }

        #vis mod #mod_ident {
            use super::#ident;
            #(
                #[derive(#bevy::prelude::Component)]
                #[component(on_add = #variant_idents::enter_hook)]
                #[component(on_insert = #variant_idents::enter_hook)]
                #[component(on_remove = #variant_idents::exit_hook)]
                pub struct #variant_idents;

                impl #variant_idents {
                    fn enter_hook(mut world: bevy::ecs::world::DeferredWorld,
                        entity: bevy::ecs::entity::Entity,
                        id: bevy::ecs::component::ComponentId) {
                        if !world.entity(entity).contains::<#ident>() {
                            world.commands().entity(entity).remove_by_id(id);
                        }
                    }
                    fn exit_hook(mut world: bevy::ecs::world::DeferredWorld,
                            entity: bevy::ecs::entity::Entity,
                            _id: bevy::ecs::component::ComponentId) {
                        if world.entity(entity).contains::<#ident>() {
                            world.commands().entity(entity).remove::<#ident>();
                        }
                    }
                }
            )*
        }
    })
}

fn get_crate(name: &str) -> proc_macro2::TokenStream {
    let found_crate = crate_name(name).expect(&format!("`{}` is present in `Cargo.toml`", name));

    match found_crate {
        FoundCrate::Itself => quote!(crate),
        FoundCrate::Name(name) => {
            let ident = format_ident!("{}", &name);
            quote!( #ident )
        }
    }
}