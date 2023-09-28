use crate::shared::structs::{DynamicAttr, TemplateConstruction, TemplateInstantiation};
use crate::TransformVisitor;
use swc_core::ecma::utils::quote_ident;
use swc_core::{
    common::{comments::Comments, DUMMY_SP},
    ecma::ast::*,
};

use super::element::AttrOptions;

pub fn create_template_dom<C: Comments>(
    mut visitor: &mut TransformVisitor<C>,
    result: &mut TemplateInstantiation,
    wrap: bool,
) -> Expr {
    if let Some(id) = result.id.clone() {
        register_template::<C>(&mut visitor, result);
        if result.exprs.is_empty()
            && result.dynamics.is_empty()
            && result.post_exprs.is_empty()
            && result.declarations.len() == 1
        {
            return *result.declarations[0].init.clone().unwrap();
        } else {
            return Expr::Call(CallExpr {
                span: DUMMY_SP,
                callee: Callee::Expr(Box::new(Expr::Arrow(ArrowExpr {
                    span: DUMMY_SP,
                    params: vec![],
                    body: Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
                        span: DUMMY_SP,
                        stmts: [Stmt::Decl(Decl::Var(Box::new(VarDecl {
                            span: DUMMY_SP,
                            kind: VarDeclKind::Const,
                            declare: false,
                            decls: result.declarations.clone(),
                        })))]
                        .into_iter()
                        .chain(result.exprs.clone().into_iter().map(|x| {
                            Stmt::Expr(ExprStmt {
                                span: DUMMY_SP,
                                expr: Box::new(x),
                            })
                        }))
                        .chain(
                            wrap_dynamics::<C>(&mut visitor, &mut result.dynamics)
                                .unwrap_or_default()
                                .into_iter()
                                .map(|x| {
                                    Stmt::Expr(ExprStmt {
                                        span: DUMMY_SP,
                                        expr: Box::new(x),
                                    })
                                }),
                        )
                        .chain(result.post_exprs.clone().into_iter().map(|x| {
                            Stmt::Expr(ExprStmt {
                                span: DUMMY_SP,
                                expr: Box::new(x),
                            })
                        }))
                        .chain([Stmt::Return(ReturnStmt {
                            span: DUMMY_SP,
                            arg: Some(Box::new(Expr::Ident(id))),
                        })])
                        .collect(),
                    })),
                    is_async: false,
                    is_generator: false,
                    type_params: None,
                    return_type: None,
                }))),
                args: vec![],
                type_args: None,
            });
        }
    }

    if wrap && result.dynamic && !visitor.config.memo_wrapper.is_empty() {
        return Expr::Call(CallExpr {
            span: DUMMY_SP,
            callee: Callee::Expr(Box::new(Expr::Ident(
                visitor.register_import_method(&visitor.config.memo_wrapper.clone()),
            ))),
            args: vec![result.exprs[0].clone().into()],
            type_args: None,
        });
    }

    result.exprs[0].clone()
}

pub fn register_template<C: Comments>(
    visitor: &mut TransformVisitor<C>,
    results: &mut TemplateInstantiation,
) {
    let decl: VarDeclarator;

    if !results.template.is_empty() {
        let template_id: Ident;
        if !results.skip_template {
            let template_def = visitor
                .templates
                .iter()
                .find(|t| t.template == results.template);
            if let Some(template_def) = template_def {
                template_id = template_def.id.clone();
            } else {
                template_id = visitor.generate_uid_identifier("tmpl$");
                visitor.templates.push(TemplateConstruction {
                    id: template_id.clone(),
                    template: results.template.clone(),
                    is_svg: results.is_svg,
                    is_ce: results.has_custom_element,
                });
            }

            decl = VarDeclarator {
                span: DUMMY_SP,
                name: Pat::Ident(results.id.clone().unwrap().into()),
                init: Some(Box::new(Expr::Call(CallExpr {
                    span: DUMMY_SP,
                    callee: Callee::Expr(Box::new(Expr::Ident(template_id))),
                    args: vec![],
                    type_args: None,
                }))),
                definite: false,
            };

            results.declarations.insert(0, decl);
        }
    }
}

fn wrap_dynamics<C: Comments>(
    visitor: &mut TransformVisitor<C>,
    dynamics: &mut Vec<DynamicAttr>,
) -> Option<Vec<Expr>> {
    if dynamics.is_empty() {
        return None;
    }

    let effect_wrapper_id = visitor.register_import_method(&visitor.config.effect_wrapper.clone());

    if dynamics.len() == 1 {
        let prev_value = if dynamics[0].key == "classList" || dynamics[0].key == "style" {
            Some(Ident::new("_$p".into(), Default::default()))
        } else {
            None
        };

        if dynamics[0].key.starts_with("class:")
            && !matches!(dynamics[0].value, Expr::Lit(Lit::Bool(_)))
            && !dynamics[0].value.is_unary()
        {
            dynamics[0].value = Expr::Unary(UnaryExpr {
                span: Default::default(),
                op: UnaryOp::Bang,
                arg: Box::new(Expr::Unary(UnaryExpr {
                    span: Default::default(),
                    op: UnaryOp::Bang,
                    arg: Box::new(dynamics[0].value.clone()),
                })),
            });
        }

        return Some(vec![Expr::Call(CallExpr {
            span: Default::default(),
            callee: Callee::Expr(Box::new(Expr::Ident(effect_wrapper_id))),
            args: vec![ExprOrSpread {
                spread: None,
                expr: Box::new(Expr::Arrow(ArrowExpr {
                    span: Default::default(),
                    params: prev_value
                        .clone()
                        .map(|v| {
                            vec![Pat::Ident(BindingIdent {
                                id: v,
                                type_ann: None,
                            })]
                        })
                        .unwrap_or_default(),
                    body: Box::new(BlockStmtOrExpr::Expr(Box::new(visitor.set_attr(
                        &dynamics[0].elem,
                        &dynamics[0].key,
                        &dynamics[0].value,
                        &AttrOptions {
                            is_svg: dynamics[0].is_svg,
                            is_ce: dynamics[0].is_ce,
                            dynamic: true,
                            prev_id: prev_value.map(Expr::Ident),
                            tag_name: dynamics[0].tag_name.clone(),
                        },
                    )))),
                    is_async: false,
                    is_generator: false,
                    type_params: None,
                    return_type: None,
                })),
            }],
            type_args: None,
        })]);
    }

    let mut decls = vec![];
    let mut statements = vec![];
    let mut identifiers = vec![];
    let prev_id = Ident::new("_p$".into(), DUMMY_SP);

    for dynamic in dynamics {
        let identifier = visitor.generate_uid_identifier("v$");
        if dynamic.key.starts_with("class:")
            && !matches!(dynamic.value, Expr::Lit(Lit::Bool(_)))
            && !dynamic.value.is_unary()
        {
            dynamic.value = Expr::Unary(UnaryExpr {
                span: Default::default(),
                op: UnaryOp::Bang,
                arg: Box::new(Expr::Unary(UnaryExpr {
                    span: Default::default(),
                    op: UnaryOp::Bang,
                    arg: Box::new(dynamic.value.clone()),
                })),
            });
        }
        identifiers.push(identifier.clone());
        decls.push(VarDeclarator {
            span: Default::default(),
            name: Pat::Ident(BindingIdent {
                id: identifier.clone(),
                type_ann: None,
            }),
            init: Some(Box::new(dynamic.value.clone())),
            definite: false,
        });

        if dynamic.key == "classList" || dynamic.key == "style" {
            let prev = Expr::Member(MemberExpr {
                span: Default::default(),
                obj: Box::new(Expr::Ident(prev_id.clone())),
                prop: MemberProp::Ident(identifier.clone()),
            });
            statements.push(Stmt::Expr(ExprStmt {
                span: Default::default(),
                expr: Box::new(Expr::Assign(AssignExpr {
                    span: Default::default(),
                    left: PatOrExpr::Expr(Box::new(prev.clone())),
                    op: AssignOp::Assign,
                    right: Box::new(visitor.set_attr(
                        &dynamic.elem,
                        &dynamic.key,
                        &Expr::Ident(identifier),
                        &AttrOptions {
                            is_svg: dynamic.is_svg,
                            is_ce: dynamic.is_ce,
                            tag_name: dynamic.tag_name.clone(),
                            dynamic: true,
                            prev_id: Some(prev),
                        },
                    )),
                })),
            }));
        } else {
            let prev = if dynamic.key.starts_with("style:") {
                Expr::Ident(identifier.clone())
            } else {
                Expr::Ident(quote_ident!("undefined"))
            };
            statements.push(Stmt::Expr(ExprStmt {
                span: Default::default(),
                expr: Box::new(Expr::Bin(BinExpr {
                    span: Default::default(),
                    left: Box::new(Expr::Bin(BinExpr {
                        span: Default::default(),
                        left: Box::new(Expr::Ident(identifier.clone())),
                        op: BinaryOp::NotEqEq,
                        right: Box::new(Expr::Member(MemberExpr {
                            span: Default::default(),
                            obj: Box::new(Expr::Ident(prev_id.clone())),
                            prop: MemberProp::Ident(identifier.clone()),
                        })),
                    })),
                    op: BinaryOp::LogicalAnd,
                    right: Box::new(visitor.set_attr(
                        &dynamic.elem,
                        &dynamic.key,
                        &Expr::Assign(AssignExpr {
                            span: Default::default(),
                            left: PatOrExpr::Expr(Box::new(Expr::Member(MemberExpr {
                                span: DUMMY_SP,
                                obj: Box::new(Expr::Ident(prev_id.clone())),
                                prop: MemberProp::Ident(identifier.clone()),
                            }))),
                            op: AssignOp::Assign,
                            right: Box::new(Expr::Ident(identifier)),
                        }),
                        &AttrOptions {
                            is_svg: dynamic.is_svg,
                            is_ce: dynamic.is_ce,
                            tag_name: "".to_string(),
                            dynamic: true,
                            prev_id: Some(prev),
                        },
                    )),
                })),
            }));
        }
    }

    Some(vec![Expr::Call(CallExpr {
        span: Default::default(),
        callee: Callee::Expr(Box::new(Expr::Ident(effect_wrapper_id))),
        args: vec![
            ExprOrSpread {
                spread: None,
                expr: Box::new(Expr::Arrow(ArrowExpr {
                    span: Default::default(),
                    params: vec![Pat::Ident(BindingIdent {
                        id: prev_id.clone(),
                        type_ann: None,
                    })],
                    body: Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
                        span: Default::default(),
                        stmts: [Stmt::Decl(Decl::Var(Box::new(VarDecl {
                            span: Default::default(),
                            kind: VarDeclKind::Const,
                            declare: false,
                            decls,
                        })))]
                        .into_iter()
                        .chain(statements)
                        .chain(
                            [Stmt::Return(ReturnStmt {
                                span: Default::default(),
                                arg: Some(Box::new(Expr::Ident(prev_id))),
                            })]
                            .into_iter(),
                        )
                        .collect(),
                    })),
                    is_async: false,
                    is_generator: false,
                    type_params: None,
                    return_type: None,
                })),
            },
            ExprOrSpread {
                spread: None,
                expr: Box::new(Expr::Object(ObjectLit {
                    span: Default::default(),
                    props: identifiers
                        .into_iter()
                        .map(|id| {
                            PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                                key: PropName::Ident(id),
                                value: Box::new(Expr::Ident(quote_ident!("undefined"))),
                            })))
                        })
                        .collect(),
                })),
            },
        ],
        type_args: None,
    })])
}
