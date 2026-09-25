"""Exercise model export checks independently from reference regeneration."""
import json
import subprocess
import tempfile
import unittest
from contextlib import chdir, redirect_stdout
from io import StringIO
from pathlib import Path
from unittest.mock import patch

import check_models
from model_catalog import catalog
from verify import verification_commands

FILES = ('manifest.json', 'weights.safetensors', 'source-lock.json', 'LICENSE', 'LICENSES_SOURCES')


class ModelVerificationTests(unittest.TestCase):
    def test_export_mode_retains_native_parity_and_other_acceptance_checks(self) -> None:
        full = verification_commands(False)
        exports = verification_commands(True)
        self.assertEqual(len(full), len(exports))
        for canonical, linux in zip(full, exports):
            if canonical == ['.venv/bin/python', 'tools/check_models.py']:
                self.assertEqual(linux, [*canonical, '--exports-only'])
            else:
                self.assertEqual(canonical, linux)
        self.assertIn(['cargo', 'test', '--release', '--locked', '--offline', '--', '--include-ignored'], exports)
        self.assertIn(['.venv/bin/python', 'tools/check_reference.py'], exports)
        self.assertIn(['.venv/bin/python', 'tools/check_quality.py'], exports)

    def test_exports_only_checks_every_export_and_creates_its_own_report_directory(self) -> None:
        entry = next(item for item in catalog() if item.model == 'en_core_web_sm')
        with tempfile.TemporaryDirectory() as temporary, chdir(temporary):
            committed = Path('assets') / f'{entry.model}-{entry.version}'
            committed.mkdir(parents=True)
            for name in FILES:
                (committed / name).write_text(name)

            def export(output: Path, model: str) -> None:
                self.assertEqual(model, entry.model)
                output.mkdir()
                for name in FILES:
                    (output / name).write_text(name)

            with patch('check_models.catalog', return_value=[entry]), \
                    patch('check_models.export', side_effect=export), \
                    patch('check_models.subprocess.run') as reference, \
                    patch('check_models.generate') as capabilities, \
                    patch('sys.argv', ['check_models.py', '--exports-only']), redirect_stdout(StringIO()):
                check_models.main()
                reference.assert_not_called()
                capabilities.assert_not_called()
            report = json.loads(Path('target/reports/model-exports.json').read_text())
            self.assertEqual(report[0]['model'], entry.model)
            self.assertEqual(set(report[0]['sha256']), set(FILES))
            self.assertFalse(Path('target/reports/model-references.json').exists())
            # Missing and changed artifacts must fail even when references are not regenerated.
            for missing in (False, True):
                for report_name in ('model-exports.json', 'model-references.json'):
                    (Path('target/reports') / report_name).write_text('stale success')
                if missing:
                    (committed / 'weights.safetensors').unlink()
                else:
                    (committed / 'weights.safetensors').write_text('corrupt')
                with patch('check_models.catalog', return_value=[entry]), \
                        patch('check_models.export', side_effect=export), \
                        patch('sys.argv', ['check_models.py', '--exports-only']), redirect_stdout(StringIO()):
                    with self.assertRaises((ValueError, FileNotFoundError)):
                        check_models.main()
                self.assertFalse(Path('target/reports/model-exports.json').exists())
                self.assertFalse(Path('target/reports/model-references.json').exists())

    def test_full_mode_checks_both_models_and_capabilities_then_clears_stale_reference_report(self) -> None:
        entries = [item for item in catalog() if item.format_version != 1]
        with tempfile.TemporaryDirectory() as temporary, chdir(temporary):
            Path('fixtures').mkdir()
            fixture = '{"cases":[{"vector":[0.5]}]}'
            for entry in entries:
                committed = Path('assets') / f'{entry.model}-{entry.version}'
                committed.mkdir(parents=True)
                for name in FILES:
                    (committed / name).write_text(name)
                size = entry.model.rsplit('_', 1)[1]
                Path(f'fixtures/model-{size}-v1.expected.json').write_text(fixture)
            Path('fixtures/model-capabilities-v1.expected.json').write_text(fixture)

            def export(output: Path, model: str) -> None:
                output.mkdir()
                for name in FILES:
                    (output / name).write_text(name)

            def regenerate(command: list[str], *, check: bool) -> subprocess.CompletedProcess[str]:
                Path(command[-1]).write_text(fixture)
                return subprocess.CompletedProcess(command, 0)

            def capabilities(output: Path) -> None:
                output.write_text(fixture)

            with patch('check_models.catalog', return_value=entries), \
                    patch('check_models.export', side_effect=export), \
                    patch('check_models.subprocess.run', side_effect=regenerate) as reference, \
                    patch('check_models.generate', side_effect=capabilities) as capability, \
                    patch('sys.argv', ['check_models.py']), redirect_stdout(StringIO()):
                check_models.main()
                self.assertEqual(reference.call_count, 2)
                capability.assert_called_once()
            reports = json.loads(Path('target/reports/model-references.json').read_text())
            self.assertEqual([item['model'] for item in reports], ['en_core_web_sm', 'en_core_web_lg', 'capabilities'])
            self.assertTrue(all(item['comparison']['numbers']['vector']['compared'] == 1 for item in reports))
            with patch('check_models.catalog', return_value=entries), \
                    patch('check_models.export', side_effect=export), \
                    patch('sys.argv', ['check_models.py', '--exports-only']), redirect_stdout(StringIO()):
                check_models.main()
            self.assertFalse(Path('target/reports/model-references.json').exists())
            self.assertEqual(len(json.loads(Path('target/reports/model-exports.json').read_text())), 2)

    def test_default_mode_rejects_reference_vector_drift(self) -> None:
        entry = next(item for item in catalog() if item.model == 'en_core_web_sm')
        with tempfile.TemporaryDirectory() as temporary, chdir(temporary):
            committed = Path('assets') / f'{entry.model}-{entry.version}'
            committed.mkdir(parents=True)
            for name in FILES:
                (committed / name).write_text(name)
            Path('fixtures').mkdir()
            Path('fixtures/model-sm-v1.expected.json').write_text('{"cases":[{"vector":[0.5756875276565552]}]}')

            def export(output: Path, model: str) -> None:
                output.mkdir()
                for name in FILES:
                    (output / name).write_text(name)

            def regenerate(command: list[str], *, check: bool) -> subprocess.CompletedProcess[str]:
                self.assertTrue(check)
                Path(command[-1]).write_text('{"cases":[{"vector":[0.575691819190979]}]}')
                return subprocess.CompletedProcess(command, 0)

            with patch('check_models.catalog', return_value=[entry]), \
                    patch('check_models.export', side_effect=export), \
                    patch('check_models.subprocess.run', side_effect=regenerate), \
                    patch('sys.argv', ['check_models.py']), redirect_stdout(StringIO()):
                with self.assertRaisesRegex(ValueError, 'absolute difference'):
                    check_models.main()
            self.assertFalse(Path('target/reports/model-references.json').exists())


if __name__ == '__main__':
    unittest.main()
