"""Keep explicit Python types and prevent type-checking escape hatches."""
import ast
import io
import tokenize
from pathlib import Path


def check_source(source: str, filename: str) -> None:
    tree = ast.parse(source, filename=filename)
    for token in tokenize.generate_tokens(io.StringIO(source).readline):
        if token.type == tokenize.COMMENT:
            comment = token.string.lower().replace(' ', '')
            if 'type:ignore' in comment or 'pyright:' in comment:
                raise ValueError(f'{filename}:{token.start[0]}: fix the type instead of suppressing checks')
    banned_names = {'Any', 'cast'}
    for node in ast.walk(tree):
        if isinstance(node, ast.ImportFrom) and node.module in ('typing', 'typing_extensions'):
            for item in node.names:
                if item.name in ('Any', 'cast', 'no_type_check'):
                    banned_names.add(item.asname or item.name)
                    raise ValueError(f'{filename}:{node.lineno}: {item.name} bypasses the typing policy')
        if isinstance(node, ast.Name) and node.id in banned_names:
            raise ValueError(f'{filename}:{node.lineno}: use concrete types and checked conversion')
        if isinstance(node, ast.Attribute) and node.attr in ('Any', 'cast', 'no_type_check'):
            raise ValueError(f'{filename}:{node.lineno}: use concrete types and checked conversion')
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            if node.returns is None:
                raise ValueError(f'{filename}:{node.lineno}: missing return type for {node.name}')
            arguments = [*node.args.posonlyargs, *node.args.args, *node.args.kwonlyargs]
            arguments += [arg for arg in (node.args.vararg, node.args.kwarg) if arg is not None]
            for arg in arguments:
                if arg.arg not in ('self', 'cls') and arg.annotation is None:
                    raise ValueError(f'{filename}:{node.lineno}: missing type for {arg.arg}')


def source_paths(root: Path) -> list[Path]:
    return sorted(path for path in (root/'tools').rglob('*')
                  if path.suffix in ('.py', '.pyi')
                  and not {'node_modules', '__pycache__'}.intersection(path.relative_to(root/'tools').parts))


def main() -> None:
    root = Path(__file__).resolve().parent.parent
    paths = source_paths(root)
    for path in paths:
        check_source(path.read_text(), str(path.relative_to(root)))
    print(f'PASS explicit typing policy for {len(paths)} Python and stub files')


if __name__ == '__main__':
    main()
