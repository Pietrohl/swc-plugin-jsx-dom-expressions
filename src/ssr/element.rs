use swc_core::{
    common::{comments::Comments, DUMMY_SP},
    ecma::ast::{
        BlockStmt, CallExpr, Callee, Expr, ExprOrSpread, GetterProp, Ident, JSXAttrOrSpread,
        JSXElement, JSXElementChild, Lit, ObjectLit, Prop, PropName, PropOrSpread, ReturnStmt,
        Stmt,
    },
};

use crate::{
    shared::{
        constants::VOID_ELEMENTS,
        structs::{
            RendererEnum, SomeTemplate, StringTemplate, TemplateInstantiation, VectorTemplate,
        },
        transform::{get_tag_name, TransformInfo},
        utils::{check_length, filter_children, trim_whitespace},
    },
    TransformVisitor,
};

impl<C> TransformVisitor<C>
where
    C: Comments,
{
    pub fn transform_element_ssr(
        &mut self,
        node: &JSXElement,
        info: &TransformInfo,
    ) -> TemplateInstantiation {
        let attributes = node.opening.attrs.clone();

        if attributes.iter().any(|attribute| match attribute {
            JSXAttrOrSpread::JSXAttr(_) => false,
            JSXAttrOrSpread::SpreadElement(_) => true,
        }) {
            return create_element(self, node, info);
        };

        let tag_name = get_tag_name(node);
        let void_tag = VOID_ELEMENTS.contains(&tag_name.as_str());

        let mut results = TemplateInstantiation {
            template: SomeTemplate::VectorTemplate(VectorTemplate(vec![])),
            template_values: vec![],
            declarations: vec![],
            exprs: vec![],
            dynamics: vec![],
            tag_name,
            // this may need some more work, don't know how to access the wontEscape from the node
            wont_escape: false,
            renderer: RendererEnum::SSR,
            ..Default::default()
        };

        if tag_name == "script" || tag_name == "style" {
            info.do_not_escape = true;
        }

        if info.top_level && self.config.hydratable {
            if tag_name == "head" {
                self.register_import_method("NoHydration");
                self.register_import_method("createComponent");
                let mut child = self.transform_element_ssr(
                    node,
                    &TransformInfo {
                        top_level: false,
                        ..*info.clone()
                    },
                );
                results.template = SomeTemplate::StringTemplate(StringTemplate::default());
                results.exprs.push(Expr::Call(CallExpr {
                    span: DUMMY_SP,
                    callee: Callee::Expr(Box::new(Expr::Ident(Ident::new(
                        "_$createComponent".into(),
                        DUMMY_SP,
                    )))),
                    args: vec![
                        ExprOrSpread {
                            spread: None,
                            expr: Box::new(Expr::Ident(Ident::new(
                                "_$NoHydration".into(),
                                DUMMY_SP,
                            ))),
                        },
                        ExprOrSpread {
                            spread: None,
                            expr: Box::new(Expr::Object(ObjectLit {
                                span: DUMMY_SP,
                                props: vec![PropOrSpread::Prop(Box::new(Prop::Getter(
                                    GetterProp {
                                        span: DUMMY_SP,
                                        key: PropName::Ident(Ident::new(
                                            "children".into(),
                                            DUMMY_SP,
                                        )),
                                        type_ann: None,
                                        body: Some(BlockStmt {
                                            span: DUMMY_SP,
                                            stmts: vec![Stmt::Return(ReturnStmt {
                                                span: DUMMY_SP,
                                                arg: Some(Box::new(
                                                    self.create_template_ssr(&mut child),
                                                )),
                                            })],
                                        }),
                                    },
                                )))],
                            })),
                        },
                    ],
                    type_args: None,
                }));

                return results;
            }

            results.template = SomeTemplate::VectorTemplate(VectorTemplate(vec!["".into()]));
            results.template_values.push(Expr::Call(CallExpr {
                span: DUMMY_SP,
                args: vec![],
                type_args: None,
                callee: Callee::Expr(Box::new(Expr::Ident(
                    self.register_import_method("ssrHydrationKey".into()),
                ))),
            }));
        }
        let mut node = node.clone();
        transform_attributes(self, &mut node, &mut results, info);
        append_to_template(&mut results.template, vec![">".into()]);
        if !void_tag {
            transform_children(self, &node, &mut results);
            append_to_template(&mut results.template, vec![format!("</{}>", tag_name)]);
        }

        return results;
    }
}

fn append_to_template(template: &mut SomeTemplate, value: Vec<String>) {
    match template {
        SomeTemplate::VectorTemplate(VectorTemplate(template)) => {
            if let Some(last) = template.last_mut() {
                if let Some(first) = value.first() {
                    last.push_str(first);
                }
            }

            if value.len() > 1 {
                template.extend(value[1..].to_vec());
            }
        }
        _ => (),
    }
}

fn create_element<C: Comments>(
    visitor: &mut TransformVisitor<C>,
    node: &JSXElement,
    info: &TransformInfo,
) -> TemplateInstantiation {
    let tag_name = get_tag_name(node);
    let attributes = normalize_attributes(visitor, node);

    let filtered_children = node
        .children
        .iter()
        .filter(|c| filter_children(c))
        .collect::<Vec<&JSXElementChild>>();

    let multi = check_length(&filtered_children);
    let markers = visitor.config.hydratable && multi;

    let child_nodes = filtered_children.iter().enumerate().fold(
        Vec::<Expr>::new(),
        |mut memo, (index, child)| {
            match child {
                JSXElementChild::JSXText(child) => {
                    let v = html_escape::decode_html_entities(&trim_whitespace(&child.raw).into());
                    if v.len() > 0 {
                        let value = Expr::Lit(Lit::Str(v.into()));
                        memo.push(value);
                    }
                }
                _ => {
                    let mut child_node = visitor.transform_node(child, info);

                    match child_node {
                        Some(child_node) => {
                            if markers && child_node.exprs.len() > 0 {
                                memo.push(Expr::Lit("<!--#-->".into()));
                            };
                            if child_node.exprs.len() > 0 {
                                child_node.exprs[0] = escape_expression(visitor, node, &child_node);
                                memo.push(visitor.create_template_ssr(&mut child_node))
                            }
                            if markers && child_node.exprs.len() > 0 {
                                memo.push(Expr::Lit("<!--/-->".into()));
                            }
                        }
                        _ => {}
                    };
                }
            };
            return memo;
        },
    );

    let props = match attributes.len() {
        1 => attributes[0].clone().node,
        _ => {}
    };

    if attributes.len() == 1 {
        let node = JSXAttrOrSpread::SpreadElement(attributes[0]);
    } else {
    };
}

fn normalize_attributes<C: Comments>(
    visitor: &mut TransformVisitor<C>,
    node: &JSXElement,
) -> Vec<JSXAttrOrSpread> {
}

fn escape_expression<C: Comments>(
    visitor: &mut TransformVisitor<C>,
    node: &JSXElement,
    child: &TemplateInstantiation,
) -> Expr {
}

fn transform_children<C: Comments>(
    visitor: &mut TransformVisitor<C>,
    node: &JSXElement,
    results: &mut TemplateInstantiation,
) {
}

fn transform_attributes<C: Comments>(
    visitor: &mut TransformVisitor<C>,
    node: &mut JSXElement,
    results: &mut TemplateInstantiation,
    info: &TransformInfo,
) {
}
