use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::visit_mut::{visit_block_mut, visit_expr_mut, VisitMut};
use syn::{
    parse_macro_input, Attribute, Block, DataStruct, DeriveInput, Expr, Ident, Type, Visibility,
};

struct RewriteVisitor {
    if_depth: usize,
}

impl RewriteVisitor {
    fn new() -> Self {
        Self { if_depth: 0 }
    }
}

fn handle_control_flow(input: impl ToTokens) -> impl ToTokens {
    quote! {
        match #input {
            lego::prelude::ControlFlow::Continue => unreachable!(),
            lego::prelude::ControlFlow::Break(v) => return lego::prelude::ControlFlow::Break(v),
            lego::prelude::ControlFlow::Ret(v) => return lego::prelude::ControlFlow::Ret(v),
            lego::prelude::ControlFlow::Preempt => return lego::prelude::ControlFlow::Preempt,
        }
    }
}

impl VisitMut for RewriteVisitor {
    fn visit_expr_return_mut(&mut self, _: &mut syn::ExprReturn) {
        unreachable!("return should be handled")
    }

    // fn visit_expr_binary_mut(&mut self, _i: &mut syn::ExprBinary) {
    //     unreachable!("binop should be rewritten");
    // }

    fn visit_expr_mut(&mut self, e: &mut syn::Expr) {
        match e {
            // conflict in implementation for integers.
            // When doing 1 + 1 for example, what should be the output?
            // This should emit a bool: we can evaluate this expression at compile time.
            // This can be solved with a new trait
            // Expr::Binary(i) => {
            //
            //     visit_expr_mut(self, &mut i.left);
            //     visit_expr_mut(self, &mut i.right);
            //
            //     let new_e = match i.op {
            //         syn::BinOp::Eq(_) => {
            //             let lhs = &i.left;
            //             let rhs = &i.right;
            //             quote! {
            //                 #lhs.eq(&#rhs)
            //             }
            //         },
            //         syn::BinOp::Lt(_) => todo!(),
            //         syn::BinOp::Le(_) => todo!(),
            //         syn::BinOp::Ne(_) => todo!(),
            //         syn::BinOp::Ge(_) => todo!(),
            //         syn::BinOp::Gt(_) => todo!(),
            //         _ => return,
            //     };
            //
            //     *e = syn::parse(new_e.into()).unwrap();
            //
            // }
            Expr::Call(call) => {
                visit_expr_mut(self, &mut call.func);
                call.args
                    .iter_mut()
                    .for_each(|arg| visit_expr_mut(self, arg));

                let callee = &call.func;
                let params = call.args.iter();
                let new_call = quote! {
                    #callee.fn_call((#(#params,)*))
                };

                *e = syn::parse(new_call.into()).unwrap();
            }
            Expr::If(i) => {
                self.if_depth += 1;
                self.visit_expr_mut(&mut i.cond);
                self.visit_block_mut(&mut i.then_branch);
                if let Some((_, ref mut else_branch)) = i.else_branch {
                    self.visit_expr_mut(else_branch);
                }

                let cond = &i.cond;
                let then = &i.then_branch;
                let alt = if let Some((_, ref e)) = i.else_branch {
                    quote! { #e }
                } else {
                    quote! { () }
                };

                self.if_depth -= 1;

                let new = quote! {
                    {
                        #[allow(unreachable_code)]
                        lego::prelude::If::new(
                            || #cond,
                            |__ctx__| lego::prelude::ControlFlow::Break(#then),
                            |__ctx__| lego::prelude::ControlFlow::Break(#alt),
                        ).eval()
                        // match r {
                        //     lego::prelude::ControlFlow::Continue => return lego::prelude::ControlFlow::Continue,
                        //     lego::prelude::ControlFlow::Break(v) => v,
                        //     lego::prelude::ControlFlow::Ret(v) => return lego::prelude::ControlFlow::Ret(v),
                        //     lego::prelude::ControlFlow::Preempt => return lego::prelude::ControlFlow::Preempt,
                        // }
                    }
                }
                .into();

                *e = syn::parse::<Expr>(new).unwrap();
            }
            Expr::Break(_) => {
                panic!("break not supported")
            }
            Expr::Continue(_) => {
                panic!("continue not supported")
            }
            Expr::Return(ret) => {
                if let Some(ref mut e) = ret.expr {
                    self.visit_expr_mut(e);
                }

                if self.if_depth != 0 {
                    let ret_e = match ret.expr {
                        Some(ref e) => quote! { #e },
                        None => quote! { () },
                    };
                    let handle_cflow = handle_control_flow(quote! { __ctx__.ret(#ret_e) });
                    let new_ret = quote! {
                        #handle_cflow
                    };
                    *e = syn::parse::<Expr>(new_ret.into()).unwrap();
                }
            }
            Expr::While(while_expr) => {
                visit_expr_mut(self, &mut while_expr.cond);
                visit_block_mut(self, &mut while_expr.body);
                let cond = &while_expr.cond;
                let body = &while_expr.body;
                let new_while = quote! {
                    {
                        lego::prelude::do_while(|__ctx__| {
                            while __ctx__.cond(|| #cond) {
                                #body
                            }

                            lego::prelude::ControlFlow::Break(())
                        })
                        // match ret {
                        //     lego::prelude::ControlFlow::Break(()) => (),
                        //     lego::prelude::ControlFlow::Ret(v) => return lego::prelude::ControlFlow::Ret(v),
                        //     lego::prelude::ControlFlow::Continue => todo!(),
                        //     lego::prelude::ControlFlow::Preempt => return lego::prelude::ControlFlow::Preempt,
                        // }
                    }
                };
                *e = syn::parse(new_while.into()).unwrap();
            }
            e => visit_expr_mut(self, e),
        }
    }

    fn visit_expr_closure_mut(&mut self, _i: &mut syn::ExprClosure) {
        panic!("can define closure in if yet");
    }

    fn visit_expr_if_mut(&mut self, _i: &mut syn::ExprIf) {
        unreachable!()
    }
}

#[proc_macro]
pub fn lego(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as Block);
    RewriteVisitor::new().visit_block_mut(&mut input);
    quote! {
        #input
    }
    .into()
}

#[proc_macro_derive(LegoBlock)]
pub fn derive_lego_block(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let res = match input.data {
        syn::Data::Struct(ref data_struct) => {
            derive_struct(data_struct, &input.attrs, &input.ident, &input.vis)
        }
        syn::Data::Enum(_) => todo!(),
        syn::Data::Union(_) => todo!(),
    };

    quote! {
        #res
    }
    .into()
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

fn derive_struct(
    s: &DataStruct,
    _attrs: &[Attribute],
    name: &Ident,
    vis: &Visibility,
) -> impl ToTokens {
    let syn::Fields::Named(ref fields) = &s.fields else {
        panic!("only named structs are supported")
    };
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
