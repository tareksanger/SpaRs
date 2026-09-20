"""Execute native installation and a separate consumer with no interpreters on PATH."""
from pathlib import Path
import subprocess
import tempfile


def main() -> None:
    binary = str(Path('installer/target/release/spars-model'))
    archive = 'assets/en_core_web_md-3.8.0-py3-none-any.whl'
    Path('target').mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='native-install-', dir='target') as root:
        result = subprocess.run([binary, 'install', '--version', '3.8.0', '--root', root,
                                 '--archive', archive], env={'PATH': ''}, capture_output=True, text=True, check=True)
        installed = result.stdout.strip()
        if not Path(installed).is_relative_to(root) or not Path(installed).is_dir():
            raise RuntimeError('Installer did not return its installed directory')
        subprocess.run([binary, 'verify', installed], env={'PATH': ''}, check=True)
        subprocess.run(['consumer/target/release/native-consumer-check', installed], env={'PATH': ''}, check=True)
        listed = subprocess.check_output([binary, 'list', '--root', root], env={'PATH': ''}, text=True)
        if listed.strip() != installed:
            raise RuntimeError('Installed model not listed')
        for arguments in [[], ['install', '--root', root], ['install', '--root', root, '--version', '9.0.0'],
                          ['install', '--root', root, '--root', root], ['unknown'], ['verify']]:
            invalid = subprocess.run([binary, *arguments], env={'PATH': ''}, capture_output=True)
            if invalid.returncode == 0:
                raise RuntimeError(f'Invalid command accepted: {arguments}')
    print('PASS native local-package installation, verification, listing, CLI errors and independent inference with empty PATH')


if __name__ == '__main__':
    main()
