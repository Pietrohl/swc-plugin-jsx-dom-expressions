use std::string;

use swc_core::{
    common::comments::Comments,
    ecma::ast::{Expr, JSXAttrOrSpread, JSXElement, JSXElementChild, Lit, SpreadElement},
};

use crate::{
    shared::{
        constants::{SVG_ELEMENTS, VOID_ELEMENTS},
        structs::{TemplateInstantiation, TemplateVector},
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
        let wrap_svg =
        info.top_level && tag_name != "svg" && SVG_ELEMENTS.contains(&tag_name.as_str()); // gonna remove this too
        let is_custom_element = tag_name.contains('-');
        let void_tag = VOID_ELEMENTS.contains(&tag_name.as_str());
        
        // config is on self.config
        let attributes = node.opening.attrs.clone();
        
        if attributes.iter().any(|attribute| match attribute {
            JSXAttrOrSpread::JSXAttr(_) => false,
            JSXAttrOrSpread::SpreadElement(_) => true,
        }) {
            return create_element(self, node, info);
        };
        
        
        let tag_name = get_tag_name(node); 



        let mut results = TemplateInstantiation<TemplateVector>{
            template: TemplateVector::new()


        };

        return results;
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
        1 => {
            attributes[0].clone().node
        },
        _ => {

        }
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
