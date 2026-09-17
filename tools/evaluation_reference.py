"""Create a new official-reference fixture without overwriting an existing one."""
import argparse
import hashlib
import json
from pathlib import Path

import spacy
import thinc


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('input', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    if args.output.exists():
        parser.error('Output exists. Use a new version; frozen expectations stay unchanged.')
    if (spacy.__version__, thinc.__version__) != ('3.8.14', '8.3.13'):
        parser.error('Use the pinned reference environment.')
    model = spacy.load('en_core_web_md')
    assert model.meta['version'] == '3.8.0'
    raw = args.input.read_bytes()
    corpus = json.loads(raw)
    cases = []
    for case in corpus['cases']:
        text = case['text']
        doc = model(text)
        def spans(items):
            return [{'start': s.start, 'end': s.end, 'label': s.label_} for s in items]
        cases.append({**case, 'tokens': [
            {'start': len(text[:t.idx].encode()), 'end': len(text[:t.idx+len(t)].encode()),
             'idx': t.idx, 'whitespace': bool(t.whitespace_), 'norm': t.norm_,
             'tag': t.tag_, 'pos': t.pos_, 'morphology': str(t.morph), 'lemma': t.lemma_,
             'head': t.head.i, 'dep': t.dep_, 'sentence_start': t.is_sent_start,
             'entity_iob': t.ent_iob_, 'entity_type': t.ent_type_} for t in doc],
            'entities': spans(doc.ents), 'sentences': spans(doc.sents),
            'noun_chunks': spans(doc.noun_chunks), 'vector': doc.vector.tolist()})
    result = {'versions': {'spacy': spacy.__version__, 'thinc': thinc.__version__},
              'model': 'en_core_web_md 3.8.0', 'input_sha256': hashlib.sha256(raw).hexdigest(),
              'cases': cases}
    args.output.write_text(json.dumps(result, ensure_ascii=False) + '\n')
    print(f'Wrote {len(cases)} cases, {sum(len(c["tokens"]) for c in cases)} tokens')


if __name__ == '__main__':
    main()
