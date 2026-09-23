"""Validate implemented official architecture semantics before extracting tensors."""
from configparser import ConfigParser
from typing import TypedDict

from reference_types import Language
from spacy.symbols import IDS


class Capabilities(TypedDict):
    language: str
    tok2vec: str
    embed: str
    encode: str
    tagger: str
    transition: str
    lemmatizer: str


def validate(nlp: Language) -> Capabilities:
    return validate_configuration(nlp.config.to_str(), nlp.vocab.vectors.mode, nlp.vocab.vectors.attr)


def validate_configuration(text: str, vector_mode: str, vector_attr: int) -> Capabilities:
    config = ConfigParser(interpolation=None)
    config.read_string(text)

    def require(section: str, key: str, expected: str) -> None:
        actual = config.get(section, key, fallback=None)
        if actual != expected:
            raise ValueError(f'Unsupported {section}.{key}: {actual!r}; expected {expected}')

    require('nlp', 'lang', '"en"')
    for hook in ('before_creation', 'after_creation', 'after_pipeline_creation'):
        require('nlp', hook, 'null')
    require('nlp.tokenizer', '@tokenizers', '"spacy.Tokenizer.v1"')
    require('nlp.vectors', '@vectors', '"spacy.Vectors.v1"')
    require('components.lemmatizer', 'mode', '"rule"')
    require('components.lemmatizer', 'overwrite', 'false')
    require('components.parser', 'learn_tokens', 'false')
    if vector_mode != 'default' or vector_attr != IDS['ORTH']:
        raise ValueError('Unsupported vector mode or lookup attribute; expected default/ORTH')
    for name in ('tok2vec', 'tagger', 'parser', 'ner', 'attribute_ruler', 'lemmatizer'):
        require(f'components.{name}', 'factory', f'"{name}"')
    for section in ('components.tok2vec.model', 'components.ner.model.tok2vec'):
        require(section, '@architectures', '"spacy.Tok2Vec.v2"')
        require(section + '.embed', '@architectures', '"spacy.MultiHashEmbed.v2"')
        require(section + '.encode', '@architectures', '"spacy.MaxoutWindowEncoder.v2"')
    require('components.tagger.model', '@architectures', '"spacy.Tagger.v2"')
    require('components.tagger.model', 'normalize', 'false')
    for name in ('tagger', 'parser'):
        require(f'components.{name}.model.tok2vec', '@architectures', '"spacy.Tok2VecListener.v1"')
        require(f'components.{name}.model.tok2vec', 'upstream', '"tok2vec"')
    for name in ('parser', 'ner'):
        section = f'components.{name}.model'
        require(section, '@architectures', '"spacy.TransitionBasedParser.v2"')
        require(section, 'state_type', f'"{name}"')
        require(section, 'extra_state_tokens', 'false')
        require(section, 'use_upper', 'true')
    return {'language': 'en', 'tok2vec': 'spacy.Tok2Vec.v2', 'embed': 'spacy.MultiHashEmbed.v2',
            'encode': 'spacy.MaxoutWindowEncoder.v2', 'tagger': 'spacy.Tagger.v2',
            'transition': 'spacy.TransitionBasedParser.v2', 'lemmatizer': 'en-rule-v1'}
