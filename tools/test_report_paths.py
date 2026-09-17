"""Check that saved diagnostics contain portable paths and preserve provenance."""
from pathlib import Path
import unittest
import tempfile
import json
from check_quality import check_portable_artifacts

from report_paths import portable_text, portable_json


class ReportPathTests(unittest.TestCase):
    def test_artifact_guard_rejects_host_paths(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'reports').mkdir()
            (root/'reports/nested').mkdir()
            report = root/'reports/nested/result.json'
            report.write_text(json.dumps({'output': '/home/example/project/file'}))
            with self.assertRaisesRegex(ValueError, 'absolute filesystem paths'):
                check_portable_artifacts(root)
            report.write_text(json.dumps({'output': 'src/lib.rs'}))
            check_portable_artifacts(root)

    def test_project_paths_become_relative(self) -> None:
        root = Path.cwd()
        self.assertEqual(portable_text(f'Compiling ({root}) {root}/src/lib.rs', root),
                         'Compiling (.) src/lib.rs')

    def test_external_paths_are_removed(self) -> None:
        for prefix in ('/Users/example', '/home/example', '/private/tmp', '/var/folders/cache',
                       'C:\\Users\\example', '/Éric/private', 'file:///Users/example',
                       '\\\\server\\share'):
            with self.subTest(prefix=prefix):
                self.assertEqual(portable_text(f'error: {prefix}/file.py:2', Path.cwd()),
                                 'error: <external-path>')

    def test_xml_paths_are_removed(self) -> None:
        self.assertEqual(portable_text('</Users/example/file>', Path.cwd()), '<<external-path>>')

    def test_urls_and_relative_paths_are_preserved(self) -> None:
        text = 'https://example.org/source/file.rs tools/export.py target/model //://example.org </3'
        self.assertEqual(portable_text(text, Path.cwd()), text)

    def test_nested_reports_and_keys_are_cleaned(self) -> None:
        root = Path.cwd()
        self.assertEqual(portable_json({str(root/'file'): [str(root/'assets/model'), 3]}, root),
                         {'file': ['assets/model', 3]})


if __name__ == '__main__':
    unittest.main()
