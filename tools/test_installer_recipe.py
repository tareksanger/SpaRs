"""Check the pinned recipe, direct parameter decoding, and regeneration."""
import contextlib
import io
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import provenance

import numpy as np
from export import export as export_model

from installer_recipe import ThincSource, decode_lemmas, decode_parameter, generate, lemma_lookup_ids, lookup_digest, sha256, template
from json_types import JsonValue, json_object, json_string, read_json


class InstallerRecipeTests(unittest.TestCase):
    def test_export_removes_obsolete_notice_and_keeps_resource_notices(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / 'export'
            output.mkdir()
            (output / 'Python.txt').write_text('obsolete notice')
            with contextlib.redirect_stdout(io.StringIO()):
                export_model(output)
            self.assertEqual({path.name for path in output.iterdir()}, {
                'manifest.json', 'weights.safetensors', 'source-lock.json',
                'spacy-MIT.txt', 'thinc-MIT.txt', 'Unicode.txt', 'LICENSE', 'LICENSES_SOURCES',
            })

    def test_lemma_lookup_ids_respect_reserved_punctuation_symbol(self) -> None:
        positions = lemma_lookup_ids()
        self.assertEqual(positions['punct'], 445)
        self.assertEqual(positions['noun'], 16833663260455505849)
        self.assertEqual(decode_lemmas({'lemma_rules': {445: [['“', '"']]}}, positions), {
            'lemma_rules': {'noun': {}, 'verb': {}, 'adj': {}, 'adv': {}, 'punct': [['“', '"']]},
        })
        recipe = json_object(read_json(Path('crates/spars-model/resources/recipe.json')))
        self.assertEqual(recipe['lemma_pos'], positions)

    def test_lookup_digest_uses_sorted_compact_utf8_without_newline(self) -> None:
        expected = sha256('{"a":["café",3],"z":{"value":1}}'.encode('utf-8'))
        self.assertEqual(lookup_digest({'z': {'value': 1}, 'a': ['café', 3]}), expected)
        self.assertEqual(lookup_digest({'a': ['café', 3], 'z': {'value': 1}}), expected)
        self.assertNotEqual(lookup_digest({'a': ['cafe', 3], 'z': {'value': 1}}), expected)
        for invalid in (None, True, 1.0):
            with self.assertRaisesRegex(ValueError, 'strings or integers'):
                lookup_digest({'nested': [invalid]})

    def test_template_removes_only_native_decoded_resources(self) -> None:
        original: dict[str, JsonValue] = {'vector_keys': {}, 'lemmas': {}, 'model': 'example'}
        self.assertEqual(template(original), {'model': 'example'})
        self.assertIn('vector_keys', original)
        with self.assertRaisesRegex(ValueError, 'Missing native-decoded'):
            template({'lemmas': {}})

    def test_parameter_boundary_checks_shape_container_and_dtype(self) -> None:
        source = ThincSource('component/model', 0, 'W')
        expected = np.asarray([[1, 2], [3, 4]], dtype=np.float32)
        np.testing.assert_array_equal(decode_parameter({'params': [{'W': expected}]}, source), expected)
        invalid: list[object] = [None, {}, {'params': []}, {'params': [None]}, {'params': [{}]},
                                {'params': [{'W': np.asarray([1], dtype=np.int32)}]}]
        for value in invalid:
            with self.assertRaises(ValueError):
                decode_parameter(value, source)
        with self.assertRaisesRegex(ValueError, 'Invalid parameter node'):
            decode_parameter({'params': [{'W': expected}]}, ThincSource('component/model', -1, 'W'))

    def test_committed_recipe_inventory_and_resource_hashes(self) -> None:
        root = Path('crates/spars-model/resources')
        recipe = json_object(read_json(root / 'recipe.json'))
        self.assertEqual(json_object(recipe['identity'])['resource_revision'], 2)
        self.assertNotIn('Python.txt', json_object(recipe['resources']))
        self.assertFalse((root / 'Python.txt').exists())
        sources = json_object(recipe['sources'])
        self.assertEqual(len(sources), 69)
        self.assertEqual(json_object(sources['vectors'])['kind'], 'npy')
        self.assertEqual(sum(json_object(value)['kind'] == 'thinc' for value in sources.values()), 68)
        self.assertEqual(sha256((root / 'manifest-template.json').read_bytes()), recipe['template_sha256'])
        for name, digest in json_object(recipe['resources']).items():
            self.assertEqual(sha256((root / name).read_bytes()), json_string(digest), name)
        manifest = json_object(read_json(root / 'manifest-template.json'))
        self.assertNotIn('lemmas', manifest)
        self.assertNotIn('vector_keys', manifest)
        self.assertEqual(set(sources), set(json_object(manifest['tensors'])))
        exported = json_object(read_json(Path('assets/en_core_web_md-3.8.0/manifest.json')))
        lookups = json_object(recipe['lookup_sha256'])
        self.assertEqual(set(lookups), {'vector_keys', 'lemmas'})
        for field, digest in lookups.items():
            self.assertEqual(lookup_digest(exported[field]), digest, field)

    def test_regeneration_is_byte_identical_and_refuses_overwrite(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / 'nested' / 'resources'
            with contextlib.redirect_stdout(io.StringIO()):
                generate(output)
            committed = Path('crates/spars-model/resources')
            self.assertEqual({p.name for p in output.iterdir()}, {p.name for p in committed.iterdir()})
            for path in output.iterdir():
                self.assertEqual(path.read_bytes(), (committed / path.name).read_bytes(), path.name)
            with self.assertRaisesRegex(ValueError, 'must be empty'):
                generate(output)

    def test_installation_bookkeeping_does_not_change_recipe(self) -> None:
        observed = provenance.record()
        files = observed['en_core_web_md']['files']
        for name in ('INSTALLER', 'REQUESTED', 'direct_url.json', 'uv_cache.json'):
            files.pop('en_core_web_md-3.8.0.dist-info/' + name, None)
        files['en_core_web_md-3.8.0.dist-info/uv_cache.json'] = '0' * 64
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / 'resources'
            with patch('provenance.record', return_value=observed), contextlib.redirect_stdout(io.StringIO()):
                generate(output)
            for path in output.iterdir():
                self.assertEqual(path.read_bytes(), (Path('crates/spars-model/resources') / path.name).read_bytes(), path.name)

    def test_regeneration_rejects_changed_upstream_source_before_writing(self) -> None:
        observed = provenance.record()
        observed['spacy']['files']['spacy/tokenizer.pyx'] = '0' * 64
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / 'resources'
            with patch('provenance.record', return_value=observed), contextlib.redirect_stdout(io.StringIO()):
                with self.assertRaisesRegex(ValueError, 'differ from the pinned reference'):
                    generate(output)
            self.assertFalse(output.exists())


if __name__ == '__main__':
    unittest.main()
