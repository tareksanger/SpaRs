"""Execute native installation and a separate consumer with no interpreters on PATH."""
from pathlib import Path
import subprocess
import tempfile
from model_catalog import catalog


def main() -> None:
    binary = str(Path('installer/target/release/spars-model'))
    Path('target').mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='native-install-', dir='target') as root:
        directories: list[str] = []
        for entry in catalog():
            result = subprocess.run([binary, 'install', '--model', entry.model, '--version', entry.version, '--root', root,
                                     '--archive', str(entry.wheel)], env={'PATH': ''}, capture_output=True, text=True, check=True)
            installed = result.stdout.strip()
            if not Path(installed).is_relative_to(root) or not Path(installed).is_dir():
                raise RuntimeError('Installer did not return its installed directory')
            subprocess.run([binary, 'verify', installed], env={'PATH': ''}, check=True)
            subprocess.run(['consumer/target/release/native-consumer-check', installed], env={'PATH': ''}, check=True)
            directories.append(installed)
        listed = subprocess.check_output([binary, 'list', '--root', root], env={'PATH': ''}, text=True)
        if sorted(listed.splitlines()) != sorted(directories):
            raise RuntimeError('Installed models not listed')
        for arguments in [[], ['install', '--root', root], ['install', '--root', root, '--version', '9.0.0'],
                          ['install', '--root', root, '--root', root], ['unknown'], ['verify']]:
            invalid = subprocess.run([binary, *arguments], env={'PATH': ''}, capture_output=True)
            if invalid.returncode == 0:
                raise RuntimeError(f'Invalid command accepted: {arguments}')
    print('PASS native local-package installation, verification, listing, CLI errors and independent inference with empty PATH')


if __name__ == '__main__':
    main()
