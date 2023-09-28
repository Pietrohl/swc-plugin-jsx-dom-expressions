use super::transform::VarBindingCollector;
use crate::{config::Config, dom::template::create_template_dom};
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
};
use swc_core::{
    common::{comments::Comments, Span, DUMMY_SP},
    ecma::{
        ast::*,
        minifier::eval::Evaluator,
        utils::{prepend_stmt, private_ident},
    },
};

pub struct TemplateConstruction {
    pub template: String,
    pub id: Ident,
    pub is_svg: bool,
    pub is_ce: bool,
}

#[derive(Clone, Debug)]
pub struct DynamicAttr {
    pub elem: Ident,
    pub key: String,
    pub value: Expr,
    pub is_svg: bool,
    pub is_ce: bool,
    pub tag_name: String,
}

#[derive(Debug, Default)]
pub struct TemplateInstantiation {
    pub component: bool,
    pub template: String,
    pub declarations: Vec<VarDeclarator>,
    pub id: Option<Ident>,
    pub tag_name: String,
    pub exprs: Vec<Expr>,
    pub dynamics: Vec<DynamicAttr>,
    pub post_exprs: Vec<Expr>,
    pub is_svg: bool,
    pub is_void: bool,
    pub has_custom_element: bool,
    pub text: bool,
    pub dynamic: bool,
    pub to_be_closed: Option<HashSet<String>>,
    pub skip_template: bool,
}

pub struct TransformVisitor<C>
where
    C: Comments,
{
    pub config: Config,
    pub template: Option<TemplateInstantiation>,
    pub templates: Vec<TemplateConstruction>,
    pub imports: HashMap<String, Ident>,
    pub events: HashSet<String>,
    pub comments: C,
    pub evaluator: Option<Evaluator>,
    pub binding_collector: VarBindingCollector,
    uid_identifier_map: HashMap<String, usize>,
}

impl<C> TransformVisitor<C>
where
    C: Comments,
{
    pub fn new(config: Config, comments: C) -> Self {
        Self {
            config,
            templates: vec![],
            template: None,
            imports: Default::default(),
            events: Default::default(),
            comments,
            evaluator: Default::default(),
            binding_collector: VarBindingCollector::new(),
            uid_identifier_map: HashMap::new(),
        }
    }

    pub fn generate_uid_identifier(&mut self, name: &str) -> Ident {
        let name = if name.starts_with('_') {
            name.to_string()
        } else {
            "_".to_string() + name
        };
        if let Some(count) = self.uid_identifier_map.get_mut(&name) {
            *count += 1;
            private_ident!(format!("{name}{count}"))
        } else {
            self.uid_identifier_map.insert(name.clone(), 1);
            private_ident!(name)
        }
    }

    pub fn create_template(&mut self, result: &mut TemplateInstantiation, wrap: bool) -> Expr {
        create_template_dom(self, result, wrap)
    }

    pub fn append_templates(&mut self, module: &mut Module) {
        if self.templates.is_empty() {
            return;
        }
        let templ = self.register_import_method("template");
        prepend_stmt(
            &mut module.body,
            ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
                span: DUMMY_SP,
                kind: VarDeclKind::Const,
                declare: false,
                decls: self
                    .templates
                    .drain(..)
                    .map(|template| {
                        let span = Span::dummy_with_cmt();
                        self.comments.add_pure_comment(span.lo);
                        let mut args = vec![ExprOrSpread {
                            spread: None,
                            expr: Box::new(
                                Tpl {
                                    span: DUMMY_SP,
                                    exprs: vec![],
                                    quasis: vec![TplElement {
                                        span: DUMMY_SP,
                                        tail: true,
                                        cooked: None,
                                        raw: template.template.into(),
                                    }],
                                }
                                .into(),
                            ),
                        }];
                        if template.is_svg || template.is_ce {
                            args.push(ExprOrSpread {
                                spread: None,
                                expr: Box::new(Expr::Lit(template.is_ce.into())),
                            });
                            args.push(ExprOrSpread {
                                spread: None,
                                expr: Box::new(Expr::Lit(template.is_svg.into())),
                            });
                        }
                        VarDeclarator {
                            span: DUMMY_SP,
                            name: template.id.into(),
                            init: Some(Box::new(Expr::Call(CallExpr {
                                span,
                                callee: Callee::Expr(Box::new(Expr::Ident(templ.clone()))),
                                args,
                                type_args: None,
                            }))),
                            definite: false,
                        }
                    })
                    .collect(),
            })))),
        )
    }
}

pub struct ProcessSpreadsInfo {
    pub elem: Option<Ident>,
    pub is_svg: bool,
    pub has_children: bool,
    pub wrap_conditionals: bool,
}
