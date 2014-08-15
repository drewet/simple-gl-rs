use std::fmt;
use std::from_str::FromStr;
use std::gc::Gc;
use syntax::ast;
use syntax::ext::base;
use syntax::ext::build::AstBuilder;
use syntax::ext::deriving::generic;
use syntax::{attr, codemap};
use syntax::parse::token;

/// Expand #[vertex_format]
pub fn expand(ecx: &mut base::ExtCtxt, span: codemap::Span,
              meta_item: Gc<ast::MetaItem>, item: Gc<ast::Item>,
              push: |Gc<ast::Item>|)
{
    generic::TraitDef {
        span: span,
        attributes: Vec::new(),
        path: generic::ty::Path {
            path: vec!["simple_gl", "VertexFormat"],
            lifetime: None,
            params: Vec::new(),
            global: true,
        },
        additional_bounds: Vec::new(),
        generics: generic::ty::LifetimeBounds::empty(),
        methods: vec![
            generic::MethodDef {
                name: "build_bindings",
                generics: generic::ty::LifetimeBounds::empty(),
                explicit_self: None,
                args: vec![
                    generic::ty::Literal(generic::ty::Path {
                        path: vec!["Option"],
                        lifetime: None,
                        params: vec![box generic::ty::Self],
                        global: false,
                    })
                ],
                ret_ty: generic::ty::Literal(
                    generic::ty::Path::new(
                        vec!["simple_gl", "VertexBindings"]
                    ),
                ),
                attributes: Vec::new(),
                combine_substructure: generic::combine_substructure(body),
            },
        ],
    }.expand(ecx, meta_item, item, push);
}

fn body(ecx: &mut base::ExtCtxt, span: codemap::Span,
        substr: &generic::Substructure) -> Gc<ast::Expr>
{
    let ecx: &base::ExtCtxt = ecx;

    match substr.fields {
        &generic::StaticStruct(ref definition, generic::Named(ref fields)) => {
            let content = definition.fields.iter().zip(fields.iter())
                .map(|(def, &(ident, _))| {
                    let elem_type = def.node.ty;
                    let ident_str = token::get_ident(ident);
                    let ident_str = ident_str.get();

                    quote_expr!(ecx, {
                        let elem: $elem_type = unsafe { mem::uninitialized() };

                        bindings.insert($ident_str.to_string(), (
                            GLDataTuple::get_gl_type(None::<$elem_type>),
                            GLDataTuple::get_num_elems(None::<$elem_type>),
                            offset_sum
                        ));

                        offset_sum += mem::size_of::<$elem_type>();
                    })

                }).collect::<Vec<Gc<ast::Expr>>>();

            quote_expr!(ecx, {
                use simple_gl::GLDataTuple;
                use std::mem;

                let mut bindings = { use std::collections::HashMap; HashMap::new() };
                let mut offset_sum = 0;
                $content;
                bindings
            })
        },

        _ => {
            ecx.span_err(span, "Unable to implement `simple_gl::VertexFormat::build_bindings` \
                                on a non-structure");
            ecx.expr_lit(span, ast::LitNil)
        }
    }
}
