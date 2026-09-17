from reference_types import Doc, Vocab
from typing import TypedDict, NotRequired

class StringOperator(TypedDict, total=False):
    IN: list[str]
    NOT_IN: list[str]
    IS_SUPERSET: list[str]
    INTERSECTS: list[str]

class DependencyNode(TypedDict):
    RIGHT_ID: str
    RIGHT_ATTRS: dict[str, str | StringOperator]
    LEFT_ID: NotRequired[str]
    REL_OP: NotRequired[str]

class DependencyMatcher:
    def __init__(self, vocab: Vocab, *, validate: bool = False) -> None: ...
    def add(self, key: str, patterns: list[list[DependencyNode]]) -> None: ...
    def __call__(self, doc: Doc) -> list[tuple[int, list[int]]]: ...
