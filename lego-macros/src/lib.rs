use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{parse_macro_input, Attribute, Block, DataStruct, DeriveInput, ExprIf, ExprWhile, Ident, Type, Visibility};
use syn::visit_mut::VisitMut;
use syn::parse::Parse;

struct LegoIfThenElse {
    i: ExprIf,
}

impl Parse for LegoIfThenElse {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let i = input.parse::<ExprIf>()?;
        Ok(Self { i })
    }
}

impl ToTokens for LegoIfThenElse {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let cond = &self.i.cond;
        let then_branch = &self.i.then_branch;
        match &self.i.else_branch {
            Some((_, else_branch)) => {
                quote! {
                    lego::prelude::If::new(|| #cond)
                        .then(lego::prelude::Then(|| #then_branch))
                        .alt(lego::prelude::Else(|| #else_branch))
                }.to_tokens(tokens);
            }
            None => {
                quote! {
                    lego::prelude::If::new(|| #cond)
                        .then(lego::prelude::Then(|| #then_branch))
                        .finish()
                }.to_tokens(tokens);
            }
        }
    }
}

struct LegoWhile {
    w: ExprWhile,
}

impl Parse for LegoWhile {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let w = input.parse::<ExprWhile>()?;
        Ok(Self { w })
    }
}

impl ToTokens for LegoWhile {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let cond = &self.w.cond;
        let body = &self.w.body;
        quote! {
            lego::prelude::While::new(|| #cond)
                .body(lego::prelude::Body(|| #body))
        }.to_tokens(tokens);
    }
}

struct RewriteVisitor {
    in_if: bool,
}

impl RewriteVisitor {
    fn new() -> Self {
        Self { in_if: false }
    }
}

impl VisitMut for RewriteVisitor {
    fn visit_expr_return_mut(&mut self, _i: &mut syn::ExprReturn) {
        if self.in_if {
            panic!("can't have return yet")
        }
    }

    fn visit_expr_if_mut(&mut self, i: &mut syn::ExprIf) {
        self.in_if = true;
        self.visit_expr_mut(&mut i.cond);
        self.visit_block_mut(&mut i.then_branch);
        if let Some((_, ref mut else_branch)) = i.else_branch {
            self.visit_expr_mut(else_branch);
        }
        self.in_if = false;

        let cond = &i.cond;
        let then = &i.then_branch;
        let alt = if let Some((_, ref e)) = i.else_branch {
            quote! {
                lego::prelude::Else(|| #e)
            }
        } else {
            quote! {
                lego::prelude::Else(lego::prelude::Never)
            }
        };

        let new = quote! {
            if true {
                (|| #cond).eval(lego::prelude::Then(|| #then), #alt)
            } else {
                unreachable!()
            }
        }.into();

        *i = syn::parse::<ExprIf>(new).unwrap();
    }
}

#[proc_macro]
pub fn lego(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as Block);
    RewriteVisitor::new().visit_block_mut(&mut input);
    quote! { #input }.into()
}

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
