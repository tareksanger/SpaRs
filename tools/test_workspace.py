"""Check Cargo's resolved workspace boundaries and shared build directory."""
import json
from pathlib import Path
import subprocess
import unittest

from json_types import json_array, json_object, json_string, validate_json


class WorkspaceTests(unittest.TestCase):
    def test_members_share_the_root_and_consumer_stays_independent(self) -> None:
        root = Path(__file__).resolve().parent.parent
        for manifest in ('Cargo.toml', 'crates/spars/Cargo.toml', 'crates/spars-model/Cargo.toml', 'bindings/node/Cargo.toml', 'consumer/Cargo.toml'):
            with self.subTest(manifest=manifest):
                result = subprocess.check_output([
                    'cargo', 'metadata', '--offline', '--locked', '--no-deps', '--format-version', '1',
                    '--manifest-path', manifest,
                ], cwd=root, text=True)
                data = json_object(validate_json(json.loads(result)))
                consumer = manifest.startswith('consumer/')
                expected_root = root / 'consumer' if consumer else root
                self.assertEqual(Path(json_string(data['workspace_root'])), expected_root)
                self.assertEqual(Path(json_string(data['target_directory'])), expected_root / 'target')
                members = {json_string(item) for item in json_array(data['workspace_members'])}
                names = {
                    json_string(package['name'])
                    for item in json_array(data['packages'])
                    if json_string((package := json_object(item))['id']) in members
                }
                self.assertEqual(names, {'native-consumer-check'} if consumer else {'spars-nlp', 'spars-model', 'spars-node'})
                defaults = {json_string(item) for item in json_array(data['workspace_default_members'])}
                default_names = {
                    json_string(package['name'])
                    for item in json_array(data['packages'])
                    if json_string((package := json_object(item))['id']) in defaults
                }
                expected_default = 'native-consumer-check' if consumer else {
                    'crates/spars-model/Cargo.toml': 'spars-model',
                    'bindings/node/Cargo.toml': 'spars-node',
                }.get(manifest, 'spars-nlp')
                self.assertEqual(default_names, {expected_default})



if __name__ == '__main__':
    unittest.main()
