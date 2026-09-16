"""Export Python Unicode classifications and official lexical resources."""
import unicodedata
from spacy.lang import lex_attrs
from spacy.lang.en import lex_attrs as en_attrs

def export(nlp):
    names=['alpha','digit','lower','upper','title','punct','currency','case_ignorable','regex_word','decimal','space']
    ranges={k:[] for k in names}
    for i in range(0x110000):
        c=chr(i);category=unicodedata.category(c)
        values=[c.isalpha(),c.isdigit(),c.islower(),c.isupper(),category=='Lt',category.startswith('P'),category=='Sc',('AΣ'+c).lower()[1]=='ς' and ('AΣ'+c+'A').lower()[1]=='σ',c.isalnum() or c=='_',c.isdecimal(),c.isspace()]
        for k,on in zip(names,values):
            if on:
                if ranges[k] and ranges[k][-1][1]==i-1:ranges[k][-1][1]=i
                else:ranges[k].append([i,i])
    constants={name:list(next(v for v in getattr(lex_attrs,name).__code__.co_consts if isinstance(v,tuple))) for name in ('is_bracket','is_quote','is_left_punct','is_right_punct')}
    return {'lower':{chr(i):chr(i).lower() for i in range(0x110000) if chr(i)!=chr(i).lower()},'unicode_version':unicodedata.unidata_version,'ranges':ranges,'stops':sorted(nlp.Defaults.stop_words),
        'number_words':en_attrs._num_words+en_attrs._ordinal_words,'tlds':sorted(lex_attrs._tlds),
        'email_regex':lex_attrs._like_email.__self__.pattern,**constants}


def translate_pattern(pattern, lexical):
    """Translate Python shorthand classes into pinned explicit Unicode ranges."""
    if pattern is None:return None
    def body(name):
        return ''.join(r'\u{%x}' % a if a==b else r'\u{%x}-\u{%x}' % (a,b) for a,b in lexical['ranges'][name])
    result=[];inside=False;i=0
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
