import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';
import { loadModel, type Document, type Span } from '@spars/node';

function spanText(doc: Document, span: Span): string {
  const first = doc.tokens[span.start];
  const last = doc.tokens[span.end - 1];
  if (!first || !last) throw new Error('Invalid token span returned by the model');
  return doc.text.slice(first.utf16Start, last.utf16End);
}

async function main(): Promise<void> {
  const selection = process.env.SPARS_MODEL
    ?? fileURLToPath(new URL('../../../../assets/en_core_web_sm-3.8.0', import.meta.url));
  console.log('Loading SpaRs model…');
  const model = await loadModel(selection);
  console.log('Ready. Native Rust inference through @spars/node.');

  async function analyze(text: string): Promise<void> {
    const start = performance.now();
    const doc = await model.process(text);
    console.log(`\n${JSON.stringify(doc.text)} (${(performance.now() - start).toFixed(1)} ms)`);
    console.table(doc.tokens.map(token => ({
      index: token.index, text: token.text, lemma: token.lemma,
      pos: token.pos, tag: token.tag, dependency: token.dep, head: token.head,
    })));
    console.log('Entities:');
    if (doc.entities === null) console.log('Unavailable');
    else if (doc.entities.length === 0) console.log('(none)');
    else console.table(doc.entities.map(span => ({ text: spanText(doc, span), label: span.label })));
    console.log('Sentences:', doc.sentences?.map(span => spanText(doc, span)) ?? 'Unavailable');
    console.log('Noun chunks:', doc.nounChunks?.map(span => spanText(doc, span)) ?? 'Unavailable');
  }

  const args = process.argv.slice(2);
  if (args.length > 0) {
    await analyze(args.join(' '));
    return;
  }
  await analyze('Alice works at Microsoft in London.');
  console.log('\nEnter text, then press Return. Type :quit to exit.');
  const input = createInterface({ input: process.stdin, output: process.stdout });
  try {
    if (process.stdin.isTTY) process.stdout.write('> ');
    for await (const line of input) {
      if (line === ':quit') break;
      await analyze(line);
      if (process.stdin.isTTY) process.stdout.write('> ');
    }
  } finally {
    input.close();
  }
}

main().catch((error: unknown) => {
  console.error(error instanceof Error ? error.message : String(error));
  console.error('Build bindings/node and provide a local export or installed model via SPARS_MODEL. See README.md.');
  process.exitCode = 1;
});
