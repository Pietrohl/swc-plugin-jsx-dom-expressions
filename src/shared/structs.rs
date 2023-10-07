use super::transform::VarBindingCollector;
use crate::config::Config;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
};
use swc_core::{
    common::comments::Comments,
    ecma::{ast::*, minifier::eval::Evaluator, utils::private_ident},
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

#[derive(Debug, Default, Clone)]
pub struct StringTemplate(pub String);
#[derive(Debug, Default, Clone)]
pub struct VectorTemplate(Vec<String>);

#[derive(Debug, Clone)]
pub enum SomeTemplate {
    StringTemplate(StringTemplate),
    VectorTemplate(VectorTemplate),
}

impl Default for SomeTemplate {
    fn default() -> Self {
        SomeTemplate::StringTemplate(StringTemplate(String::new()))
    }
}
impl PartialEq for SomeTemplate {
    fn eq(&self, other: &Self) -> bool {
        // check of both are of the same type and if so compare the inner values
        match (self, other) {
            (
                SomeTemplate::StringTemplate(StringTemplate(ref string1)),
                SomeTemplate::StringTemplate(StringTemplate(ref string2)),
            ) => string1 == string2,
            (
                SomeTemplate::VectorTemplate(VectorTemplate(ref vec1)),
                SomeTemplate::VectorTemplate(VectorTemplate(ref vec2)),
            ) => vec1 == vec2,
            _ => false,
        }
    }
}

impl Into<String> for SomeTemplate {
    fn into(self) -> String {
        match self {
            SomeTemplate::StringTemplate(StringTemplate(string)) => string,
            SomeTemplate::VectorTemplate(VectorTemplate(vec)) => vec.join(""),
        }
    }
}


impl SomeTemplate {
    pub fn append(&mut self, s: &str) {
        match self {
            SomeTemplate::StringTemplate(StringTemplate(ref mut string)) => {
                string.push_str(s);
            }
            SomeTemplate::VectorTemplate(VectorTemplate(ref mut vec)) => {
                vec.push(s.to_owned());
            }
        }
    }

    // prepend a string to the template either as a string or as a vector
    pub fn prepend(&mut self, s: &str) {
        match self {
            SomeTemplate::StringTemplate(StringTemplate(ref mut string)) => {
                string.insert_str(0, s);
            }
            SomeTemplate::VectorTemplate(VectorTemplate(ref mut vec)) => {
                vec.insert(0, s.to_owned());
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            SomeTemplate::StringTemplate(StringTemplate(ref string)) => string.is_empty(),
            SomeTemplate::VectorTemplate(VectorTemplate(ref vec)) => vec.is_empty(),
        }
    }

    pub fn append_template(&mut self, template: &SomeTemplate) {
        match template {
            SomeTemplate::StringTemplate(StringTemplate(ref string)) => {
                self.append(string);
            }
            _ => {
                panic!("append_template: not implemented for VectorTemplate");
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct TemplateInstantiation {
    pub component: bool,
    pub template: SomeTemplate,
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

        self.create_template_dom(result, wrap)
        // match self.config.generate.as_str() {
        //     "ssr" => self.create_template_ssr(result),
        //     _ => self.create_template_dom(result, wrap),
        // }
    }

    pub fn append_templates(&mut self, module: &mut Module) {
        self.append_templates_dom(module)

        // match self.config.generate.as_str() {
        //     "ssr" => self.append_templates_ssr(module),
        //     _ => self.append_templates_dom(module),
        // }
    }
}

pub struct ProcessSpreadsInfo {
    pub elem: Option<Ident>,
    pub is_svg: bool,
    pub has_children: bool,
    pub wrap_conditionals: bool,
}
