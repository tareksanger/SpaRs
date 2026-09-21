"""Regenerate the pinned native installer recipe from official model resources."""
import argparse
from collections.abc import Mapping, Sequence
from dataclasses import asdict, dataclass
import hashlib
import importlib
import io
import json
from pathlib import Path
import tempfile
from typing import Literal, Protocol, TypeGuard, runtime_checkable
import zipfile

import numpy as np
from safetensors.numpy import save_file
import spacy
from spacy.strings import hash_string
from spacy.symbols import IDS

from export import export
from installer_provenance import pinned_source_lock
from json_types import JsonValue, json_object, json_string, read_json, validate_json
from reference_types import FloatArray, Language, Model, is_float_array

WHEEL = Path('assets/en_core_web_md-3.8.0-py3-none-any.whl')
WHEEL_SHA256 = '5e6329fe3fecedb1d1a02c3ea2172ee0fede6cea6e4aefb6a02d832dba78a310'
PREFIX = 'en_core_web_md/en_core_web_md-3.8.0/'


@dataclass(frozen=True)
class ThincSource:
    entry: str
    node: int
    parameter: str
    kind: Literal['thinc'] = 'thinc'


@dataclass(frozen=True)
class NpySource:
    entry: str
    kind: Literal['npy'] = 'npy'


@dataclass(frozen=True)
class Identity:
    model: str = 'en_core_web_md'
    model_version: str = '3.8.0'
    wheel_sha256: str = WHEEL_SHA256
    recipe_revision: int = 1
    resource_revision: int = 2
    format_version: int = 1


@dataclass(frozen=True)
class LookupDigests:
    vector_keys: str
    lemmas: str


@dataclass(frozen=True)
class Recipe:
    identity: Identity
    sources: dict[str, ThincSource | NpySource]
    inputs: dict[str, str]
    lemma_pos: dict[str, int]
    vector_keys_entry: str
    lemmas_entry: str
    template_sha256: str
    resources: dict[str, str]
    field_sources: dict[str, str]
    lookup_sha256: LookupDigests


@runtime_checkable
class MsgpackDecoder(Protocol):
    def msgpack_loads(self, data: bytes) -> object: ...


def mapping(value: object) -> TypeGuard[Mapping[object, object]]:
    return isinstance(value, dict)


def sequence(value: object) -> TypeGuard[Sequence[object]]:
    return isinstance(value, list)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def lookup_digest(value: JsonValue) -> str:
    """Hash sorted compact UTF-8 JSON; prohibit ambiguous floating-point values."""
    def validate(item: JsonValue) -> None:
        if isinstance(item, dict):
            for child in item.values():
                validate(child)
        elif isinstance(item, list):
            for child in item:
                validate(child)
        elif isinstance(item, bool) or not isinstance(item, (str, int)):
            raise ValueError('Lookup values must be strings or integers')
    validate(value)
    return sha256(json.dumps(value, ensure_ascii=False, separators=(',', ':'), sort_keys=True).encode('utf-8'))


def lemma_lookup_ids() -> dict[str, int]:
    # Table.get uses get_string_id, which checks reserved symbols before hashing.
    return {pos: IDS.get(pos, hash_string(pos)) for pos in ('noun', 'verb', 'adj', 'adv', 'punct')}


def decode_lemmas(data: object, positions: dict[str, int]) -> dict[str, dict[str, JsonValue]]:
    if not mapping(data):
        raise ValueError('Lemma tables must be a mapping')
    result: dict[str, dict[str, JsonValue]] = {}
    for name, table in data.items():
        if not isinstance(name, str) or not mapping(table):
            raise ValueError('Invalid named lemma table')
        result[name] = {pos: validate_json(table.get(key, {})) for pos, key in positions.items()}
    return result


def model_sources(nlp: Language) -> dict[str, ThincSource | NpySource]:
    """Use the same semantic model references as export.py, never tensor values."""
    sources: dict[str, ThincSource | NpySource] = {}

    def component(root: Model, name: str) -> tuple[list[Model], str]:
        return list(root.walk()), PREFIX + name + '/model'

    def params(model: Model, prefix: str, nodes: list[Model], entry: str) -> None:
        node = next(i for i, candidate in enumerate(nodes) if candidate is model)
        for parameter in model.param_names:
            key = prefix + '.' + parameter
            if key in sources:
                raise ValueError('Duplicate tensor source: ' + key)
            sources[key] = ThincSource(entry, node, parameter)

    def tok2vec(model: Model, prefix: str, nodes: list[Model], entry: str) -> None:
        embed = list(model.get_ref('embed').walk())
        encode = list(model.get_ref('encode').walk())
        for i, node in enumerate(x for x in embed if x.name == 'hashembed'):
            params(node, f'{prefix}.hash{i}', nodes, entry)
        params(next(x for x in embed if x.name == 'static_vectors'), prefix + '.static', nodes, entry)
        params(next(x for x in embed if x.name == 'maxout'), prefix + '.mix.max', nodes, entry)
        params(next(x for x in embed if x.name == 'layernorm'), prefix + '.mix.norm', nodes, entry)
        for i, (maximum, norm) in enumerate(zip(
            (x for x in encode if x.name == 'maxout'),
            (x for x in encode if x.name == 'layernorm'), strict=True,
        )):
            params(maximum, f'{prefix}.enc{i}.max', nodes, entry)
            params(norm, f'{prefix}.enc{i}.norm', nodes, entry)

    shared = nlp.get_pipe('tok2vec').model
    tok2vec(shared, 'tok2vec', *component(shared, 'tok2vec'))
    tagger = nlp.get_pipe('tagger').model
    params(tagger.layers[1].layers[0], 'tagger', *component(tagger, 'tagger'))
    for name in ('parser', 'ner'):
        model = nlp.get_pipe(name).model
        nodes, entry = component(model, name)
        params(model.get_ref('tok2vec').layers[-1], name + '.reduce', nodes, entry)
        params(model.get_ref('lower'), name + '.lower', nodes, entry)
        params(model.get_ref('upper'), name + '.upper', nodes, entry)
        if name == 'ner':
            tok2vec(model.get_ref('tok2vec').layers[0], 'ner.tok2vec', nodes, entry)
    sources['vectors'] = NpySource(PREFIX + 'vocab/vectors')
    return sources


def decode_parameter(data: object, source: ThincSource) -> FloatArray:
    if not mapping(data):
        raise ValueError('Model must be a mapping')
    parameters = data.get('params')
    if not sequence(parameters) or not 0 <= source.node < len(parameters):
        raise ValueError('Invalid parameter node')
    node = parameters[source.node]
    if not mapping(node):
        raise ValueError('Parameter node must be a mapping')
    array = node.get(source.parameter)
    if not is_float_array(array):
        raise ValueError('Expected float32 parameter')
    return np.ascontiguousarray(array)


def reconstruct(wheel: zipfile.ZipFile, sources: dict[str, ThincSource | NpySource]) -> dict[str, FloatArray]:
    decoder: object = importlib.import_module('srsly')
    if not isinstance(decoder, MsgpackDecoder):
        raise TypeError('srsly must expose msgpack_loads')
    decoded: dict[str, object] = {}
    tensors: dict[str, FloatArray] = {}
    for key, source in sources.items():
        if isinstance(source, ThincSource):
            if source.entry not in decoded:
                decoded[source.entry] = decoder.msgpack_loads(wheel.read(source.entry))
            tensors[key] = decode_parameter(decoded[source.entry], source)
        else:
            array: object = np.load(io.BytesIO(wheel.read(source.entry)), allow_pickle=False)
            if not is_float_array(array):
                raise ValueError('Expected float32 vectors')
            tensors[key] = np.ascontiguousarray(array)
    return tensors


def template(manifest: dict[str, JsonValue]) -> dict[str, JsonValue]:
    result = dict(manifest)
    for field in ('vector_keys', 'lemmas'):
        if field not in result:
            raise ValueError('Missing native-decoded field: ' + field)
        del result[field]
    return result


def write_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, ensure_ascii=False, separators=(',', ':'), sort_keys=True) + '\n')


def generate(output: Path) -> None:
    if output.exists() and any(output.iterdir()):
        raise ValueError('Output directory must be empty; compare regenerated resources explicitly')
    if sha256(WHEEL.read_bytes()) != WHEEL_SHA256:
        raise ValueError('Official wheel checksum mismatch')
    with tempfile.TemporaryDirectory(prefix='spars-recipe-') as temporary:
        work = Path(temporary)
        export(work / 'export')
        manifest = json_object(read_json(work / 'export/manifest.json'))
        sources = model_sources(spacy.load('en_core_web_md'))
        if set(sources) != set(json_object(manifest['tensors'])) or len(sources) != 69:
            raise ValueError('Tensor source inventory differs from the official export')
        with zipfile.ZipFile(WHEEL) as wheel:
            save_file(reconstruct(wheel, sources), str(work / 'weights.safetensors'))
            if sha256((work / 'weights.safetensors').read_bytes()) != json_string(manifest['weights_sha256']):
                raise ValueError('Direct wheel tensors differ from official exported tensors')
            decoder: object = importlib.import_module('srsly')
            if not isinstance(decoder, MsgpackDecoder):
                raise TypeError('srsly must expose msgpack_loads')
            lemma_data = decoder.msgpack_loads(wheel.read(PREFIX + 'lemmatizer/lookups/lookups.bin'))
            if decode_lemmas(lemma_data, lemma_lookup_ids()) != manifest['lemmas']:
                raise ValueError('Direct wheel lemma lookups differ from the official export')
            entries = {source.entry for source in sources.values()} | {
                PREFIX + 'vocab/key2row', PREFIX + 'lemmatizer/lookups/lookups.bin',
                PREFIX + 'LICENSE', PREFIX + 'LICENSES_SOURCES',
            }
            inputs = {entry: sha256(wheel.read(entry)) for entry in sorted(entries)}
        source_lock = pinned_source_lock(read_json(work / 'export/source-lock.json'))
        output.mkdir(parents=True, exist_ok=True)
        write_json(output / 'manifest-template.json', template(manifest))
        resources: dict[str, str] = {}
        for notice in ('source-lock.json', 'spacy-MIT.txt', 'thinc-MIT.txt', 'Unicode.txt'):
            data = source_lock if notice == 'source-lock.json' else (work / 'export' / notice).read_bytes()
            (output / notice).write_bytes(data)
            resources[notice] = sha256(data)
        field_sources = {field: 'Official exporter tools/export.py using pinned spaCy 3.8.14, Thinc 8.3.13 and model 3.8.0' for field in template(manifest)}
        field_sources.update({
            'lexical': 'tools/lexical.py: official spaCy English resources and Python/Unicode character properties',
            'tokenizer': 'Official model tokenizer rules and patterns; tools/lexical.py regex translation',
            'norms': 'Official spaCy BASE_NORMS plus model vocab/lookups.bin lexeme_norm',
            'symbols': 'Official spaCy symbols.IDS',
            'attribute_rules': 'Official model attribute_ruler/patterns',
            'vector_keys': PREFIX + 'vocab/key2row',
            'lemmas': PREFIX + 'lemmatizer/lookups/lookups.bin',
        })
        recipe = Recipe(Identity(), sources, inputs,
            lemma_lookup_ids(),
            PREFIX + 'vocab/key2row', PREFIX + 'lemmatizer/lookups/lookups.bin',
            sha256((output / 'manifest-template.json').read_bytes()), resources, field_sources,
            LookupDigests(lookup_digest(manifest['vector_keys']), lookup_digest(manifest['lemmas'])))
        write_json(output / 'recipe.json', asdict(recipe))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    output: object = parser.parse_args().out
    if not isinstance(output, Path):
        raise TypeError('Output must be a path')
    generate(output)


if __name__ == '__main__':
    main()
