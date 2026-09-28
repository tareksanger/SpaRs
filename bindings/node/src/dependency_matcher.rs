//! Reusable dependency matching over immutable native documents.
use crate::{
    document::NativeDocument,
    errors,
    matcher_types::{invalid, TokenConstraint},
};
use napi::{
    bindgen_prelude::{AsyncTask, Utf16String},
    Env, Result, Task,
};
use napi_derive::napi;
use std::sync::Arc;

#[napi(object)]
pub struct DependencyLink {
    pub left: Utf16String,
    /// One of the twenty supported spaCy dependency relation symbols.
    pub relation: Utf16String,
}

#[napi(object)]
pub struct DependencyNode {
    pub id: Utf16String,
    pub constraints: Vec<TokenConstraint>,
    /// Absent on the first node; later nodes link to an earlier node ID.
    pub link: Option<DependencyLink>,
}

#[napi(object)]
pub struct DependencyPattern {
    pub nodes: Vec<DependencyNode>,
}

#[napi(object, object_from_js = false)]
pub struct DependencyMatch {
    pub rule: String,
    /// Token indices in pattern node order.
    #[napi(ts_type = "Array<import('./units.js').TokenIndex>")]
    pub tokens: Vec<u32>,
}

fn relation(value: &str) -> errors::Result<spars::Relation> {
    use spars::Relation::*;
    Ok(match value {
        "<" => Parent,
        ">" => Child,
        "<<" => Ancestor,
        ">>" => Descendant,
        "." => ImmediatelyPrecedes,
        ".*" => Precedes,
        ";" => ImmediatelyFollows,
        ";*" => Follows,
        "$+" => ImmediateRightSibling,
        "$-" => ImmediateLeftSibling,
        "$++" => RightSibling,
        "$--" => LeftSibling,
        ">+" => ImmediateRightChild,
        ">-" => ImmediateLeftChild,
        ">++" => RightChild,
        ">--" => LeftChild,
        "<+" => ImmediateRightParent,
        "<-" => ImmediateLeftParent,
        "<++" => RightParent,
        "<--" => LeftParent,
        _ => return Err(invalid("unknown dependency relation")),
    })
}

fn relation_name(value: spars::Relation) -> &'static str {
    use spars::Relation::*;
    match value {
        Parent => "<",
        Child => ">",
        Ancestor => "<<",
        Descendant => ">>",
        ImmediatelyPrecedes => ".",
        Precedes => ".*",
        ImmediatelyFollows => ";",
        Follows => ";*",
        ImmediateRightSibling => "$+",
        ImmediateLeftSibling => "$-",
        RightSibling => "$++",
        LeftSibling => "$--",
        ImmediateRightChild => ">+",
        ImmediateLeftChild => ">-",
        RightChild => ">++",
        LeftChild => ">--",
        ImmediateRightParent => "<+",
        ImmediateLeftParent => "<-",
        RightParent => "<++",
        LeftParent => "<--",
    }
}

impl DependencyPattern {
    fn into_native(self) -> errors::Result<spars::DependencyPattern> {
        let nodes = self
            .nodes
            .into_iter()
            .map(|node| {
                Ok(spars::DependencyNode {
                    id: errors::text(&node.id)?,
                    constraints: node
                        .constraints
                        .into_iter()
                        .map(TokenConstraint::into_native)
                        .collect::<errors::Result<Vec<_>>>()?,
                    link: node
                        .link
                        .map(|link| -> errors::Result<spars::DependencyLink> {
                            Ok(spars::DependencyLink {
                                left: errors::text(&link.left)?,
                                relation: relation(&errors::text(&link.relation)?)?,
                            })
                        })
                        .transpose()?,
                })
            })
            .collect::<errors::Result<Vec<_>>>()?;
        Ok(spars::DependencyPattern { nodes })
    }

    fn from_native(pattern: &spars::DependencyPattern) -> Self {
        Self {
            nodes: pattern
                .nodes
                .iter()
                .map(|node| DependencyNode {
                    id: node.id.clone().into(),
                    constraints: node
                        .constraints
                        .iter()
                        .map(TokenConstraint::from_native)
                        .collect(),
                    link: node.link.as_ref().map(|link| DependencyLink {
                        left: link.left.clone().into(),
                        relation: relation_name(link.relation).to_owned().into(),
                    }),
                })
                .collect(),
        }
    }
}

#[derive(Default)]
#[napi]
pub struct DependencyMatcher {
    inner: Arc<spars::DependencyMatcher>,
}

#[napi]
impl DependencyMatcher {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(spars::DependencyMatcher::new()),
        }
    }

    #[napi(getter)]
    pub fn size(&self, env: Env) -> Result<u32> {
        errors::number(self.inner.len()).map_err(|error| error.into_napi(env))
    }

    #[napi(strict)]
    pub fn contains(&self, env: Env, rule: Utf16String) -> Result<bool> {
        let rule = errors::text(&rule).map_err(|error| error.into_napi(env))?;
        Ok(self.inner.contains(&rule))
    }

    /// Return owned patterns, or null when the rule is absent.
    #[napi(strict)]
    pub fn get(&self, env: Env, rule: Utf16String) -> Result<Option<Vec<DependencyPattern>>> {
        let rule = errors::text(&rule).map_err(|error| error.into_napi(env))?;
        Ok(self.inner.get(&rule).map(|patterns| {
            patterns
                .iter()
                .map(DependencyPattern::from_native)
                .collect()
        }))
    }

    /// Validate the complete addition before appending to an existing rule.
    #[napi(strict)]
    pub fn add(
        &mut self,
        env: Env,
        rule: Utf16String,
        patterns: Vec<DependencyPattern>,
    ) -> Result<()> {
        let rule = errors::text(&rule).map_err(|error| error.into_napi(env))?;
        let matcher = Arc::get_mut(&mut self.inner).ok_or_else(|| errors::busy().into_napi(env))?;
        let patterns = patterns
            .into_iter()
            .map(DependencyPattern::into_native)
            .collect::<errors::Result<Vec<_>>>()
            .map_err(|error| error.into_napi(env))?;
        matcher
            .add(rule, patterns)
            .map_err(|error| errors::BindingError::from(error).into_napi(env))
    }

    #[napi(strict)]
    pub fn remove(&mut self, env: Env, rule: Utf16String) -> Result<()> {
        let rule = errors::text(&rule).map_err(|error| error.into_napi(env))?;
        let matcher = Arc::get_mut(&mut self.inner).ok_or_else(|| errors::busy().into_napi(env))?;
        matcher
            .remove(&rule)
            .map_err(|error| errors::BindingError::from(error).into_napi(env))
    }

    /// Search on Node's worker pool in native rule, pattern and candidate order.
    #[napi(strict, ts_return_type = "Promise<Array<DependencyMatch>>")]
    pub fn find_matches(&self, document: &NativeDocument) -> AsyncTask<DependencyMatchTask> {
        AsyncTask::new(DependencyMatchTask {
            matcher: Arc::clone(&self.inner),
            document: Arc::clone(&document.inner),
        })
    }
}

pub struct DependencyMatchTask {
    matcher: Arc<spars::DependencyMatcher>,
    document: Arc<spars::Doc>,
}

#[napi]
impl Task for DependencyMatchTask {
    type Output = errors::Result<Vec<DependencyMatch>>;
    type JsValue = Vec<DependencyMatch>;

    fn compute(&mut self) -> Result<Self::Output> {
        Ok(self
            .matcher
            .find_matches(&self.document)
            .map_err(Into::into)
            .and_then(|matches| {
                matches
                    .into_iter()
                    .map(|matched| {
                        Ok(DependencyMatch {
                            rule: matched.rule,
                            tokens: matched
                                .tokens
                                .into_iter()
                                .map(|token| errors::number(token.0))
                                .collect::<errors::Result<Vec<_>>>()?,
                        })
                    })
                    .collect()
            }))
    }

    fn resolve(&mut self, env: Env, output: Self::Output) -> Result<Self::JsValue> {
        output.map_err(|error| error.into_napi(env))
    }
}
