use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{parse_macro_input, Attribute, DataStruct, DeriveInput, Ident, Type, Visibility};

#[proc_macro_derive(LegoBlock)]
pub fn derive_lego_block(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let res = match input.data {
        syn::Data::Struct(ref data_struct) => derive_struct(data_struct, &input.attrs, &input.ident, &input.vis),
        syn::Data::Enum(_) => todo!(),
        syn::Data::Union(_) => todo!(),
    };

    quote! {
        #res
    }.into()
}

fn trait_proxy_fn_sig(vis: &Visibility, name: &Ident, ty: &Type) -> impl ToTokens {
    quote! {
        #vis fn #name(&self) -> lego::prelude::Proxy<#ty>
    }
}

fn trait_proxy_mut_fn_sig(vis: &Visibility, name: &Ident, ty: &Type) -> impl ToTokens {
    let name = format_ident!("{name}_mut");
    quote! {
        #vis fn #name(&self) -> lego::prelude::ProxyMut<#ty>
    }
}

fn derive_struct(s: &DataStruct, _attrs: &[Attribute], name: &Ident, vis: &Visibility) -> impl ToTokens {
    let syn::Fields::Named(ref fields) = &s.fields else { panic!("only named structs are supported") };
    let proxy_trait_ident = format_ident!("{name}Proxy");
    let proxy_trait_ident_mut = format_ident!("{name}ProxyMut");

    let trait_proxy_fns = fields.named.iter().map(|f| {
        let sig = trait_proxy_fn_sig(vis, f.ident.as_ref().unwrap(), &f.ty);
        quote! { #sig; }
    });

    let trait_proxy_fns_mut = fields.named.iter().map(|f| {
        let sig = trait_proxy_mut_fn_sig(vis, f.ident.as_ref().unwrap(), &f.ty);
        quote! { #sig; }
    });

    let trait_proxy_impls = fields.named.iter().map(|f| {
        let field = f.ident.as_ref().unwrap();
        let sig = trait_proxy_fn_sig(vis, field, &f.ty);
        quote! {
            #sig {
                lego::prelude::Proxy::new(self.addr(), self.offset() + std::mem::offset_of!(#name, #field) as i32)
            }
        }
    });

    let trait_proxy_mut_impls = fields.named.iter().map(|f| {
        let field = f.ident.as_ref().unwrap();
        let sig = trait_proxy_mut_fn_sig(vis, field, &f.ty);
        quote! {
            #sig {
                lego::prelude::ProxyMut::new(self.addr(), self.offset() + std::mem::offset_of!(#name, #field) as i32)
            }
        }
    });

    quote! {
        trait #proxy_trait_ident {
            #(#trait_proxy_fns)*
        }

        trait #proxy_trait_ident_mut {
            #(#trait_proxy_fns_mut)*
        }

        impl #proxy_trait_ident for lego::prelude::Proxy<#name> {
            #(#trait_proxy_impls)*
        }

        impl #proxy_trait_ident_mut for lego::prelude::ProxyMut<#name> {
            #(#trait_proxy_mut_impls)*
        }

        unsafe impl lego::prelude::JitSafe for #name {}
    }
}
