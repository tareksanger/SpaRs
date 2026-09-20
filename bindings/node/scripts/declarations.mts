import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';

/** napi-rs omits a constructor declaration for classes created by worker tasks. */
export function restrictConstructor(source: string): string {
  const declaration = /export declare class Model \{([^}]*)\}/g;
  const matches = [...source.matchAll(declaration)];
  const match = matches[0];
  if (matches.length !== 1 || !match || match[1] === undefined) throw new Error('Expected one generated Model class');
  if (match[1].includes('private constructor()')) return source;
  if (match[1].includes('constructor(')) throw new Error('Review changed native Model constructor');
  return source.replace('export declare class Model {', 'export declare class Model {\n  private constructor()');
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const path = new URL('../index.d.ts', import.meta.url);
  writeFileSync(path, restrictConstructor(readFileSync(path, 'utf8')));
}
