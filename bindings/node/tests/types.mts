import { Model } from '../index.js';
import type { Document, Token, Span, Stage } from '../index.js';
import type { ByteOffset, CodePointOffset, Utf16Offset, TokenIndex } from '../units.js';

type Assert<T extends true> = T;
type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends (<T>() => T extends B ? 1 : 2) ? true : false;
type Distinct<A, B> = A extends B ? false : true;

export type Contracts = [
  Assert<Distinct<typeof Model, new () => Model>>,
  Assert<Equal<Awaited<ReturnType<Model['process']>>, Document>>,
  Assert<Equal<Awaited<ReturnType<Model['processBatch']>>, Document[]>>,
  Assert<Equal<Document['entities'], Span[] | null>>,
  Assert<Equal<Token['head'], TokenIndex | null>>,
  Assert<Equal<Token['byteStart'], ByteOffset>>,
  Assert<Equal<Token['codePointStart'], CodePointOffset>>,
  Assert<Equal<Token['utf16Start'], Utf16Offset>>,
  Assert<Distinct<ByteOffset, Utf16Offset>>,
  Assert<Distinct<CodePointOffset, TokenIndex>>,
  Assert<Distinct<'unknown', Stage>>,
  Assert<Equal<ReturnType<Model['vector']>, Float32Array | null>>,
];
