"""Check that stable recipe provenance still detects changed upstream sources."""
from pathlib import Path
import unittest
from installer_provenance import pinned_source_lock, source_records
from json_types import json_object, read_json


class InstallerProvenanceTests(unittest.TestCase):
    def test_verified_linux_source_variant_preserves_pinned_recipe(self) -> None:
        path = Path('reference/source-lock.json')
        data = json_object(read_json(path))
        record = json_object(data['spacy'])
        files = json_object(record['files'])
        files['spacy/matcher/levenshtein.c'] = 'e14722922674055c3d72e21fd8b727597c58ba7395f70be68eaa55aa15bd44a9'
        self.assertEqual(pinned_source_lock(data), path.read_bytes())
        # Only the verified file at the pinned version may use this alternate hash.
        files['spacy/matcher/levenshtein.c'] = '0' * 64
        with self.assertRaises(ValueError):
            pinned_source_lock(data)

    def test_linux_variant_does_not_allow_other_source_or_version_changes(self) -> None:
        for field in ('version', 'source'):
            data = json_object(read_json(Path('reference/source-lock.json')))
            record = json_object(data['spacy'])
            files = json_object(record['files'])
            files['spacy/matcher/levenshtein.c'] = 'e14722922674055c3d72e21fd8b727597c58ba7395f70be68eaa55aa15bd44a9'
            if field == 'version':
                record['version'] = '3.8.13'
            else:
                files['spacy/tokenizer.pyx'] = files['spacy/matcher/levenshtein.c']
            with self.assertRaises(ValueError):
                pinned_source_lock(data)

    def test_unchanged_capture_keeps_every_original_byte(self) -> None:
        path = Path('reference/source-lock.json')
        self.assertEqual(pinned_source_lock(read_json(path)), path.read_bytes())

    def test_source_checksum_version_license_and_inventory_changes_fail(self) -> None:
        for package, field in [('spacy', 'version'), ('thinc', 'release'), ('en_core_web_md', 'files')]:
            data = json_object(read_json(Path('reference/source-lock.json')))
            record = json_object(data[package])
            if field == 'files':
                files = json_object(record[field])
                files['en_core_web_md/en_core_web_md-3.8.0/LICENSE'] = '0' * 64
            else:
                record[field] = 'changed'
            with self.assertRaisesRegex(ValueError, 'differ from the pinned'):
                pinned_source_lock(data)
        data = json_object(read_json(Path('reference/source-lock.json')))
        del data['murmurhash']
        with self.assertRaises(ValueError):
            pinned_source_lock(data)

    def test_unknown_metadata_is_not_silently_ignored(self) -> None:
        data = json_object(read_json(Path('reference/source-lock.json')))
        record = json_object(data['en_core_web_md'])
        json_object(record['files'])['en_core_web_md-3.8.0.dist-info/unknown.json'] = '0' * 64
        with self.assertRaises(ValueError):
            pinned_source_lock(data)
        with self.assertRaises(ValueError):
            source_records({'spacy': {'version': '3.8.14'}})
