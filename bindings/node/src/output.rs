use napi_derive::napi;

/// An owned result. Changing it in JavaScript cannot affect the native model.
#[napi(object, object_from_js = false, use_nullable = true)]
pub struct Document {
    pub text: String,
    pub tokens: Vec<Token>,
    pub entities: Option<Vec<Span>>,
    pub sentences: Option<Vec<Span>>,
    pub noun_chunks: Option<Vec<Span>>,
}

#[napi(object, object_from_js = false, use_nullable = true)]
pub struct Token {
    #[napi(ts_type = "import('./units.js').TokenIndex")]
    pub index: u32,
    pub text: String,
    /// Exactly one trailing ASCII space, or empty. Other whitespace has its own token.
    pub whitespace: String,
    #[napi(ts_type = "import('./units.js').ByteOffset")]
    pub byte_start: u32,
    #[napi(ts_type = "import('./units.js').ByteOffset")]
    pub byte_end: u32,
    #[napi(ts_type = "import('./units.js').CodePointOffset")]
    pub code_point_start: u32,
    #[napi(ts_type = "import('./units.js').CodePointOffset")]
    pub code_point_end: u32,
    #[napi(ts_type = "import('./units.js').Utf16Offset")]
    pub utf16_start: u32,
    #[napi(ts_type = "import('./units.js').Utf16Offset")]
    pub utf16_end: u32,
    pub norm: String,
    pub tag: Option<String>,
    pub pos: Option<String>,
    pub morphology: Option<String>,
    pub lemma: Option<String>,
    #[napi(ts_type = "import('./units.js').TokenIndex")]
    pub head: Option<u32>,
    pub dep: Option<String>,
    pub sentence_start: Option<bool>,
    pub entity_iob: Option<String>,
    pub entity_type: Option<String>,
}

/// A half-open token interval: start is included; end is excluded.
#[napi(object, object_from_js = false)]
pub struct Span {
    #[napi(ts_type = "import('./units.js').TokenIndex")]
    pub start: u32,
    #[napi(ts_type = "import('./units.js').TokenIndex")]
    pub end: u32,
    pub label: String,
}
