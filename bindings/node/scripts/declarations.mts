import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';

/** napi-rs omits a constructor declaration for classes created by worker tasks. */
export function restrictConstructor(source: string, name = 'Model'): string {
  const declaration = new RegExp(`export declare class ${name} \\{([^}]*)\\}`, 'g');
  const matches = [...source.matchAll(declaration)];
  const match = matches[0];
  if (matches.length !== 1 || !match || match[1] === undefined) throw new Error(`Expected one generated ${name} class`);
  if (match[1].includes('private constructor()')) return source;
  if (match[1].includes('constructor(')) throw new Error(`Review changed native ${name} constructor`);
  return source.replace(`export declare class ${name} {`, `export declare class ${name} {\n  private constructor()`);
}

const executionTypes = `
/** Inference admission limits shared by models in this package instance and JS isolate. */
export interface ExecutionOptions {
  maxActive: number
  maxQueued: number
}
/** Configure admission while idle. Defaults to 2 active and 32 queued; null restores these defaults. */
export declare function configureExecution(options: ExecutionOptions | null): void
/** Text lengths count UTF-16 units, matching JavaScript string.length. */
export interface InputLimits {
  maxTextLength: number
  maxBatchSize: number
  maxBatchTextLength: number
}
/** Enable additional server input limits while idle. Disabled by default; null resets them. */
export declare function configureInputLimits(options: InputLimits | null): void
export interface PipeOptions {
  batchSize?: number
  stage?: Stage | null
}
export interface Model {
  /** Lazily buffer and yield documents in order; native inference stays sequential within a batch. */
  pipe(texts: Iterable<string> | AsyncIterable<string>, options?: PipeOptions): AsyncGenerator<Document, void, unknown>
}
`;

export function addExecutionTypes(source: string): string {
  return source.includes('export interface ExecutionOptions') ? source : source + executionTypes;
}

export function addExecutionLoader(source: string): string {
  const marker = "require('./execution.cjs').install(module.exports);";
  if (source.includes(marker)) return source;
  if (!source.includes('module.exports.Model = nativeBinding.Model')) throw new Error('Expected native Model export');
  return source + `\n${marker}\nmodule.exports.configureExecution = require('./execution.cjs').configureExecution;\nmodule.exports.configureInputLimits = require('./execution.cjs').configureInputLimits;\n`;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const path = new URL('../index.d.ts', import.meta.url);
  let declarations = readFileSync(path, 'utf8');
  for (const name of ['Model', 'NativeDocument', 'NativeToken', 'NativeSpan']) declarations = restrictConstructor(declarations, name);
  writeFileSync(path, addExecutionTypes(declarations));
  const loader = new URL('../index.js', import.meta.url);
  let source = addExecutionLoader(readFileSync(loader, 'utf8'));
  for (const name of ['PhraseMatcher', 'TokenMatcher', 'DependencyMatcher']) {
    const hook = `require('./execution.cjs').installMatcher(module.exports.${name}, module.exports.NativeDocument, '${name.replace('Matcher', '').toLowerCase()}');`;
    if (!source.includes(hook)) source += `\n${hook}\n`;
  }
  writeFileSync(loader, source);
}
