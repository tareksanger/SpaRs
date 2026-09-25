//! Versioned execution semantics, independent of a model package's identity.
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Capabilities {
    pub language: Language,
    pub tok2vec: Tok2Vec,
    pub embed: Embedding,
    pub encode: Encoder,
    pub tagger: Tagger,
    pub transition: Transition,
    pub lemmatizer: Lemmatizer,
}

// Each enum identifies implemented semantics, not a list of accepted model names.
// Deserialization rejects missing or unknown capabilities before loading tensors.
#[derive(Deserialize)]
pub(crate) enum Language {
    #[serde(rename = "en")]
    English,
}
#[derive(Deserialize)]
pub(crate) enum Tok2Vec {
    #[serde(rename = "spacy.Tok2Vec.v2")]
    V2,
}
#[derive(Deserialize)]
pub(crate) enum Embedding {
    #[serde(rename = "spacy.MultiHashEmbed.v2")]
    V2,
}
#[derive(Deserialize)]
pub(crate) enum Encoder {
    #[serde(rename = "spacy.MaxoutWindowEncoder.v2")]
    V2,
}
#[derive(Deserialize)]
pub(crate) enum Tagger {
    #[serde(rename = "spacy.Tagger.v2")]
    V2,
}
#[derive(Deserialize)]
pub(crate) enum Transition {
    #[serde(rename = "spacy.TransitionBasedParser.v2")]
    V2,
}
#[derive(Deserialize)]
pub(crate) enum Lemmatizer {
    #[serde(rename = "en-rule-v1")]
    EnglishRuleV1,
}

impl Capabilities {
    pub(crate) fn validate(&self) {
        // Exhaustive matching makes adding semantics an explicit implementation step.
        let Self {
            language: Language::English,
            tok2vec: Tok2Vec::V2,
            embed: Embedding::V2,
            encode: Encoder::V2,
            tagger: Tagger::V2,
            transition: Transition::V2,
            lemmatizer: Lemmatizer::EnglishRuleV1,
        } = self;
    }
}
