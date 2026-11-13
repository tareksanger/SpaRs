use crate::{
    document::NativeDocument,
    entity_ruler::known_fields,
    errors,
    matcher_types::{invalid, TokenConstraint},
};
use napi::{
    bindgen_prelude::{AsyncTask, JsObjectValue, Object, Utf16String},
    Env, Result, Task,
};
use napi_derive::napi;
use std::sync::Arc;

#[napi(object)]
pub struct TokenRepetition {
    pub kind: Utf16String,
    pub min: Option<f64>,
    pub max: Option<f64>,
}
/// Conditions that one token must all satisfy (empty matches any token), and how many consecutive
/// tokens the item consumes.
#[napi(object)]
pub struct TokenPatternItem {
    pub constraints: Vec<TokenConstraint>,
    pub repetition: TokenRepetition,
}
/// Items matched against consecutive tokens, like one pattern list in spaCy `Matcher.add`; must not
/// be empty.
#[napi(object)]
pub struct TokenPattern {
    pub tokens: Vec<TokenPatternItem>,
}
/// A match of `rule` covering tokens `start` (inclusive) to `end` (exclusive).
#[napi(object)]
pub struct TokenMatch {
    pub rule: String,
    #[napi(ts_type = "import('./units.js').TokenIndex")]
    pub start: u32,
    #[napi(ts_type = "import('./units.js').TokenIndex")]
    pub end: u32,
}

fn bound(value: f64) -> errors::Result<usize> {
    // Native patterns expand to at most 4096 nodes. The larger u32 boundary
    // accepts every potentially executable pattern without integer truncation.
    if !value.is_finite() || value.fract() != 0.0 || value < 0.0 || value > f64::from(u32::MAX) {
        return Err(invalid(
            "repetition bounds must be integers in 0..=4294967295",
        ));
    }
    Ok(value as usize)
}
impl TokenPattern {
    pub(crate) fn into_native(self) -> errors::Result<spars::TokenPattern> {
        Ok(spars::TokenPattern {
            tokens: self
                .tokens
                .into_iter()
                .map(|item| {
                    let kind = errors::text(&item.repetition.kind)?;
                    let repetition = if kind == "range" {
                        spars::Repetition::Range {
                            min: bound(
                                item.repetition
                                    .min
                                    .ok_or_else(|| invalid("range repetition requires min"))?,
                            )?,
                            max: item.repetition.max.map(bound).transpose()?,
                        }
                    } else {
                        if item.repetition.min.is_some() || item.repetition.max.is_some() {
                            return Err(invalid("only range repetition accepts min or max"));
                        }
                        match kind.as_str() {
                            "once" => spars::Repetition::Once,
                            "optional" => spars::Repetition::Optional,
                            "zero_or_more" => spars::Repetition::ZeroOrMore,
                            "one_or_more" => spars::Repetition::OneOrMore,
                            "negated" => spars::Repetition::Negated,
                            _ => return Err(invalid("unknown repetition kind")),
                        }
                    };
                    Ok(spars::TokenPatternItem {
                        constraints: item
                            .constraints
                            .into_iter()
                            .map(TokenConstraint::into_native)
                            .collect::<errors::Result<_>>()?,
                        repetition,
                    })
                })
                .collect::<errors::Result<_>>()?,
        })
    }
    pub(crate) fn from_native(pattern: &spars::TokenPattern) -> Self {
        Self {
            tokens: pattern
                .tokens
                .iter()
                .map(|item| {
                    let (kind, min, max) = match item.repetition {
                        spars::Repetition::Once => ("once", None, None),
                        spars::Repetition::Optional => ("optional", None, None),
                        spars::Repetition::ZeroOrMore => ("zero_or_more", None, None),
                        spars::Repetition::OneOrMore => ("one_or_more", None, None),
                        spars::Repetition::Negated => ("negated", None, None),
                        spars::Repetition::Range { min, max } => {
                            ("range", Some(min as f64), max.map(|value| value as f64))
                        }
                    };
                    TokenPatternItem {
                        constraints: item
                            .constraints
                            .iter()
                            .map(TokenConstraint::from_native)
                            .collect(),
                        repetition: TokenRepetition {
                            kind: kind.to_owned().into(),
                            min,
                            max,
                        },
                    }
                })
                .collect(),
        }
    }
}

/// The value of spaCy's `greedy` argument with this spelling. Like spaCy's error E947, any other
/// string is an error.
fn greedy(name: &str) -> errors::Result<spars::Greedy> {
    match name {
        "FIRST" => Ok(spars::Greedy::First),
        "LONGEST" => Ok(spars::Greedy::Longest),
        _ => Err(invalid(&format!(
            "[E947] greedy must be \"FIRST\" or \"LONGEST\", not {name:?}"
        ))),
    }
}

/// Read the `options` argument of `TokenMatcher.add`; omitted, `undefined` and `null` give the
/// defaults.
fn rule_options(env: Env, options: Option<Object>) -> Result<spars::TokenRuleOptions> {
    let mut native = spars::TokenRuleOptions::default();
    let Some(options) = options else {
        return Ok(native);
    };
    known_fields(&options, "TokenMatcher rule option", &["greedy"])?;
    native.greedy = options
        .get_named_property::<Option<Utf16String>>("greedy")?
        .map(|name| greedy(&errors::text(&name)?))
        .transpose()
        .map_err(|error| error.into_napi(env))?;
    Ok(native)
}

/// Finds token sequences described by per-token conditions, like spaCy's `Matcher`.
#[napi]
pub struct TokenMatcher {
    inner: Arc<spars::TokenMatcher>,
}
#[napi]
impl TokenMatcher {
    /// Pass a model to enable lexical flag conditions with its language rules.
    #[napi(constructor)]
    pub fn new(model: Option<&crate::Model>) -> Self {
        let matcher = match model {
            Some(model) => spars::TokenMatcher::with_lexicon(model.inner.lexicon()),
            None => spars::TokenMatcher::new(),
        };
        Self {
            inner: Arc::new(matcher),
        }
    }
    /// Number of rules, like spaCy `len(matcher)`.
    #[napi(getter)]
    pub fn size(&self, env: Env) -> Result<u32> {
        errors::number(self.inner.len()).map_err(|error| error.into_napi(env))
    }
    #[napi(strict)]
    pub fn contains(&self, env: Env, rule: Utf16String) -> Result<bool> {
        let rule = errors::text(&rule).map_err(|error| error.into_napi(env))?;
        Ok(self.inner.contains(&rule))
    }
    /// Patterns of `rule` in addition order, or null when `rule` is not registered.
    #[napi(strict)]
    pub fn get(&self, env: Env, rule: Utf16String) -> Result<Option<Vec<TokenPattern>>> {
        let rule = errors::text(&rule).map_err(|error| error.into_napi(env))?;
        Ok(self
            .inner
            .get(&rule)
            .map(|patterns| patterns.iter().map(TokenPattern::from_native).collect()))
    }
    /// Append patterns to `rule`, like spaCy `Matcher.add`; everything is validated before the
    /// matcher changes. `options` applies to the whole rule (only `greedy` is available; spaCy's
    /// `on_match` is not): each call replaces the options of earlier calls for `rule`, so a call
    /// without `greedy` returns the rule to reporting every match. An empty `patterns` adds no
    /// pattern but still sets the options, and registers `rule` if it is new. Throws
    /// `SPARS_INVALID_PATTERN` for a pattern object of the wrong shape, an unknown `greedy`
    /// string, an empty pattern, invalid predicates or repetition bounds, a pattern expanding
    /// beyond 4096 token positions, or flag conditions without a model, checking the shape first
    /// and `greedy` second; `StringExpected` or `NumberExpected` for a pattern field of the wrong
    /// type, after the shape and before `greedy`; `SPARS_INVALID_TEXT` for a `greedy` string with an unpaired
    /// surrogate; `InvalidArg` for an unknown option or a `greedy` that is not a string; and
    /// `SPARS_BUSY` while `findMatches` is pending.
    #[napi(
        strict,
        ts_args_type = "rule: string, patterns: Array<TokenPattern>, options?: TokenRuleOptions | undefined | null"
    )]
    pub fn add(
        &mut self,
        env: Env,
        rule: Utf16String,
        patterns: Vec<TokenPattern>,
        options: Option<Object>,
    ) -> Result<()> {
        let rule = errors::text(&rule).map_err(|error| error.into_napi(env))?;
        let matcher = Arc::get_mut(&mut self.inner).ok_or_else(|| errors::busy().into_napi(env))?;
        // spaCy checks `greedy` before it looks at any pattern.
        let options = rule_options(env, options)?;
        let patterns = patterns
            .into_iter()
            .map(TokenPattern::into_native)
            .collect::<errors::Result<_>>()
            .map_err(|error| error.into_napi(env))?;
        matcher
            .add_with(rule, patterns, options)
            .map_err(|error| errors::BindingError::from(error).into_napi(env))
    }
    /// Throws `SPARS_INVALID_PATTERN` for an unregistered rule and `SPARS_BUSY` while `findMatches` is pending.
    #[napi(strict)]
    pub fn remove(&mut self, env: Env, rule: Utf16String) -> Result<()> {
        let rule = errors::text(&rule).map_err(|error| error.into_napi(env))?;
        let matcher = Arc::get_mut(&mut self.inner).ok_or_else(|| errors::busy().into_napi(env))?;
        matcher
            .remove(&rule)
            .map_err(|error| errors::BindingError::from(error).into_napi(env))
    }
    /// Matches, found on Node's worker pool, each rule, start and end once, in spaCy `Matcher`
    /// order rather than sorted by `start`: every match, overlapping ones included, of each rule
    /// without `greedy`. A rule added with `greedy` reports only matches that share no token with
    /// each other; as in spaCy, those come after the matches of every rule without `greedy`, one
    /// rule after another in the order in which the search first finds a match of each rule, and
    /// most preferred first within a rule (see `Greedy`). Rejects with `SPARS_INFERENCE` when an
    /// `equals` condition on `lemma`, `pos`, `tag`, `dep` or `morphology` meets a nonempty document
    /// in which no token has that annotation, for example `pos` before `AttributeRuler`, like
    /// spaCy's error E155. `''` counts as missing except for `morphology`. Other predicates never
    /// require the annotation, and a token without it compares as `''` (missing `morphology` has
    /// its own rules; see the Token Matcher guide). Rejects with `SPARS_UNSUPPORTED` when a
    /// condition compares `morphology` and the document's morphology is not in spaCy's canonical
    /// order, and with `SPARS_BUSY` or `SPARS_INPUT_LIMIT` under `configureExecution` and
    /// `configureInputLimits`.
    #[napi(strict, ts_return_type = "Promise<Array<TokenMatch>>")]
    pub fn find_matches(&self, doc: &NativeDocument) -> AsyncTask<TokenMatchTask> {
        AsyncTask::new(TokenMatchTask {
            matcher: Arc::clone(&self.inner),
            doc: Arc::clone(&doc.inner),
        })
    }
}
impl Default for TokenMatcher {
    fn default() -> Self {
        Self::new(None)
    }
}
pub struct TokenMatchTask {
    matcher: Arc<spars::TokenMatcher>,
    doc: Arc<spars::Doc>,
}
#[napi]
impl Task for TokenMatchTask {
    type Output = errors::Result<Vec<TokenMatch>>;
    type JsValue = Vec<TokenMatch>;
    fn compute(&mut self) -> Result<Self::Output> {
        Ok(self
            .matcher
            .find_matches(&self.doc)
            .map_err(Into::into)
            .and_then(|matches| {
                matches
                    .into_iter()
                    .map(|matched| {
                        Ok(TokenMatch {
                            rule: matched.rule,
                            start: errors::number(matched.start.0)?,
                            end: errors::number(matched.end.0)?,
                        })
                    })
                    .collect()
            }))
    }
    fn resolve(&mut self, env: Env, output: Self::Output) -> Result<Self::JsValue> {
        output.map_err(|error| error.into_napi(env))
    }
}
