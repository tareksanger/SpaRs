"""Exercise capability rejection and source verification against official packages."""
from configparser import ConfigParser
import io
import json
from pathlib import Path
import unittest
from export_capabilities import validate, validate_configuration
from installer_provenance import model_source_lock
from json_types import json_object, read_json
import spacy
from spacy.symbols import IDS


class ModelExpansionTests(unittest.TestCase):
    def test_official_models_share_native_architectures(self) -> None:
        for name in ('en_core_web_sm', 'en_core_web_md', 'en_core_web_lg'):
            model = spacy.load(name)
            self.assertEqual(validate(model)['language'], 'en')

    def test_capability_guards_reject_unimplemented_semantics(self) -> None:
        text = spacy.load('en_core_web_sm').config.to_str()
        for section, field, value in (
            ('nlp', 'lang', '"de"'),
            ('nlp', 'after_pipeline_creation', '"custom"'),
            ('nlp.tokenizer', '@tokenizers', '"custom.Tokenizer.v1"'),
            ('nlp.vectors', '@vectors', '"custom.Vectors.v1"'),
            ('components.parser', 'learn_tokens', 'true'),
            ('components.parser.model', 'extra_state_tokens', 'true'),
            ('components.ner.model', 'use_upper', 'false'),
            ('components.tagger.model', 'normalize', 'true'),
            ('components.tagger.model.tok2vec', 'upstream', '"other"'),
            ('components.tok2vec.model.encode', '@architectures', '"unknown.v1"'),
            ('components.lemmatizer', 'mode', '"lookup"'),
        ):
            config = ConfigParser(interpolation=None)
            config.read_string(text)
            config.set(section, field, value)
            output = io.StringIO()
            config.write(output)
            with self.subTest(section=section, field=field), self.assertRaisesRegex(ValueError, 'Unsupported'):
                validate_configuration(output.getvalue(), 'default', IDS['ORTH'])
        for mode, attr in [('floret', IDS['ORTH']), ('default', IDS['LOWER'])]:
            with self.assertRaisesRegex(ValueError, 'vector mode or lookup attribute'):
                validate_configuration(text, mode, attr)

    def test_new_model_provenance_is_checked_against_official_wheel(self) -> None:
        path = Path('installer/models/en_core_web_sm/source-lock.json')
        original = read_json(path)
        self.assertEqual(json.loads(model_source_lock(original, 'en_core_web_sm')), original)
        for field in ['version', 'release', 'tokenizer', 'attribute_ruler/patterns', 'LICENSE', 'inventory']:
            observed = json_object(read_json(path))
            model = json_object(observed['en_core_web_sm'])
            if field in ('version', 'release'):
                model[field] = 'changed'
            else:
                files = json_object(model['files'])
                if field == 'inventory':
                    files['unexpected.py'] = '0' * 64
                else:
                    files['en_core_web_sm/en_core_web_sm-3.8.0/' + field] = '0' * 64
                model['files'] = files
            observed['en_core_web_sm'] = model
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, 'Installed model files differ'):
                model_source_lock(observed, 'en_core_web_sm')
