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

const executionTypes = `
/** Inference admission limits shared by models in this package instance and JS isolate. */
export interface ExecutionOptions {
  maxActive: number
  maxQueued: number
}
/** Set both limits while idle. Defaults: 2 active jobs and 32 queued jobs. */
export declare function configureExecution(options: ExecutionOptions): void
/** Text lengths count UTF-16 units, matching JavaScript string.length. */
export interface InputLimits {
  maxTextLength: number
  maxBatchSize: number
  maxBatchTextLength: number
}
/** Set all input limits while idle. Defaults: 32768 units/text, 128 texts/batch, 65536 units/batch. */
export declare function configureInputLimits(options: InputLimits): void
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
  writeFileSync(path, addExecutionTypes(restrictConstructor(readFileSync(path, 'utf8'))));
  const loader = new URL('../index.js', import.meta.url);
  writeFileSync(loader, addExecutionLoader(readFileSync(loader, 'utf8')));
}
