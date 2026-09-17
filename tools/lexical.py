"""Export Python Unicode classifications and official lexical resources."""
import unicodedata
from spacy.lang import lex_attrs
from spacy.lang.en import lex_attrs as en_attrs

from typing import TypedDict
from reference_types import Language, sequence, strings
from collections.abc import Set
from typing import TypeGuard

class LexicalResources(TypedDict):
    lower: dict[str, str]
    unicode_version: str
    ranges: dict[str, list[list[int]]]
    stops: list[str]
    number_words: list[str]
    tlds: list[str]
    email_regex: str
    is_bracket: list[str]
    is_quote: list[str]
    is_left_punct: list[str]
    is_right_punct: list[str]


def string_set(value: object) -> list[str]:
    def is_set(item: object) -> TypeGuard[Set[object]]:
        return isinstance(item, (set, frozenset))
    if not is_set(value):
        raise TypeError('Expected a set of strings')
    result: list[str] = []
    for item in value:
        if not isinstance(item, str):
            raise TypeError('Expected a string set member')
        result.append(item)
    return result


def email_pattern() -> str:
    # This pinned upstream helper is a compiled regular-expression method.
    function: object = getattr(lex_attrs, '_like_email')
    owner: object = getattr(function, '__self__')
    pattern: object = getattr(owner, 'pattern')
    if not isinstance(pattern, str):
        raise TypeError('Expected the official email regex pattern')
    return pattern


def export(nlp: Language) -> LexicalResources:
    names=['alpha','digit','lower','upper','title','punct','currency','case_ignorable','regex_word','decimal','space']
    ranges: dict[str, list[list[int]]] = {k:[] for k in names}
    for i in range(0x110000):
        c=chr(i);category=unicodedata.category(c)
        values=[c.isalpha(),c.isdigit(),c.islower(),c.isupper(),category=='Lt',category.startswith('P'),category=='Sc',('AΣ'+c).lower()[1]=='ς' and ('AΣ'+c+'A').lower()[1]=='σ',c.isalnum() or c=='_',c.isdecimal(),c.isspace()]
        for k,on in zip(names,values):
            if on:
                if ranges[k] and ranges[k][-1][1]==i-1:ranges[k][-1][1]=i
                else:ranges[k].append([i,i])
    constants: dict[str, list[str]] = {}
    for name, function in [('is_bracket', lex_attrs.is_bracket), ('is_quote', lex_attrs.is_quote), ('is_left_punct', lex_attrs.is_left_punct), ('is_right_punct', lex_attrs.is_right_punct)]:
        raw: object = next(v for v in function.__code__.co_consts if sequence(v))
        if not sequence(raw) or not all(isinstance(v, str) for v in raw):
            raise TypeError(f'Expected string constants for {name}')
        constants[name] = [v for v in raw if isinstance(v, str)]
    return {'lower':{chr(i):chr(i).lower() for i in range(0x110000) if chr(i)!=chr(i).lower()},'unicode_version':unicodedata.unidata_version,'ranges':ranges,'stops':sorted(nlp.Defaults.stop_words),
        'number_words':strings(getattr(en_attrs, '_num_words'))+strings(getattr(en_attrs, '_ordinal_words')),'tlds':sorted(string_set(getattr(lex_attrs, '_tlds'))),
        'email_regex':email_pattern(),'is_bracket': constants['is_bracket'], 'is_quote': constants['is_quote'], 'is_left_punct': constants['is_left_punct'], 'is_right_punct': constants['is_right_punct']}


def translate_pattern(pattern: str | None, lexical: LexicalResources) -> str | None:
    """Translate Python shorthand classes into pinned explicit Unicode ranges."""
    if pattern is None:return None
    def body(name: str) -> str:
        return ''.join(r'\u{%x}' % a if a==b else r'\u{%x}-\u{%x}' % (a,b) for a,b in lexical['ranges'][name])
    result: list[str] = [];inside=False;i=0
    while i<len(pattern):
        c=pattern[i]
        if c=='\\' and i+1<len(pattern):
            x=pattern[i+1]
            if x in ('w','d','S'):
                value=body({'w':'regex_word','d':'decimal','S':'space'}[x])
                if x=='S':
                    assert not inside
                    result.append('[^'+value+']')
                else:result.append(value if inside else '['+value+']')
            else:result.append(pattern[i:i+2])
            i+=2;continue
        if c=='[':inside=True
        if c==']':inside=False
        result.append(c);i+=1
    return ''.join(result)
