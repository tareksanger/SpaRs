"""Direct regex span agreement; independent of tokenizer control flow."""
import json
from pathlib import Path
import spacy
from typing import TypedDict
from reference_types import RegexFunction
from json_types import json_array, json_object, json_string, read_json

class RegexCase(TypedDict):
    text: str
    prefix: list[list[int]]
    suffix: list[list[int]]
    infix: list[list[int]]
    url: list[list[int]]


def single(function: RegexFunction, text: str) -> list[list[int]]:
    match = function(text)
    return [[match.start(), match.end()]] if match else []

n=spacy.load('en_core_web_md');t=n.tokenizer
words = [json_string(json_object(row)['text']) for row in json_array(read_json(Path('fixtures/lexical.expected.json')))]
chars={word for word in words if len(word)==1}
rows: list[RegexCase] = []
for c in sorted(chars):
 for text in (c+c+'://example.org','https://example.org/'+c,'(a'+c+'b.)'):
  rows.append({'text':text,'prefix':single(t.prefix_search, text),'suffix':single(t.suffix_search, text),'infix':[[m.start(),m.end()] for m in t.infix_finditer(text)],'url':single(t.url_match, text)})
Path('fixtures/regex.expected.json').write_text(json.dumps(rows,ensure_ascii=False))
print(len(rows),'regex cases')
