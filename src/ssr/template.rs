use crate::shared::structs::TemplateInstantiation;
use crate::TransformVisitor;
use swc_core::{
    common::{comments::Comments, DUMMY_SP},
    ecma::{
        ast::{Decl, Expr, Module, ModuleItem, Stmt, VarDecl, VarDeclKind, VarDeclarator},
        utils::prepend_stmt,
    },
};

impl<C> TransformVisitor<C>
where
    C: Comments,
{
    pub fn create_template_ssr(&mut self, result: &mut TemplateInstantiation) -> Expr {
        if result.template.is_empty() {
            return result.exprs[0].clone();
        }

        Expr::Lit(result.template.clone().into())
    }

    pub fn append_templates_ssr(&mut self, module: &mut Module) {
        if self.templates.is_empty() {
            return;
        }
        prepend_stmt(
            &mut module.body,
            ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
                span: DUMMY_SP,
                kind: VarDeclKind::Const,
                declare: false,
                decls: self
                    .templates
                    .drain(..)
                    .map(|template| VarDeclarator {
                        span: DUMMY_SP,
                        name: template.id.into(),
                        init: Some(template.template.into()),
                        definite: false,
                    })
                    .collect(),
            })))),
        )
    }
}
