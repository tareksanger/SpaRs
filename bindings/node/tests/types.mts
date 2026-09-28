import { Model } from '../index.js';
import type { Document, Token, Span, Stage, NativeDocument, Lexeme, ModelStore, PhraseMatcher, PhraseMatch, TokenMatcher, TokenMatch, DependencyMatcher, DependencyMatch } from '../index.js';
import type { ByteOffset, CodePointOffset, Utf16Offset, TokenIndex } from '../units.js';

type Assert<T extends true> = T;
type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends (<T>() => T extends B ? 1 : 2) ? true : false;
type Distinct<A, B> = A extends B ? false : true;

export type Contracts = [
  Assert<Distinct<typeof Model, new () => Model>>,
  Assert<Equal<Awaited<ReturnType<Model['process']>>, Document>>,
  Assert<Equal<ReturnType<Model['processDocument']>, Promise<NativeDocument>>>,
  Assert<Equal<ReturnType<PhraseMatcher['findMatches']>, Promise<PhraseMatch[]>>>,
  Assert<Equal<ReturnType<TokenMatcher['findMatches']>, Promise<TokenMatch[]>>>,
  Assert<Equal<ReturnType<DependencyMatcher['findMatches']>, Promise<DependencyMatch[]>>>,
  Assert<Equal<ReturnType<ModelStore['resolve']>, Promise<string>>>,
  Assert<Equal<ReturnType<ModelStore['register']>, Promise<void>>>,
  Assert<Equal<ReturnType<Model['lexeme']>, Lexeme>>,
  Assert<Equal<Lexeme['orth'], bigint>>,
  Assert<Equal<ReturnType<Model['documentVector']>, Float32Array>>,
  Assert<Equal<ReturnType<Model['spanVector']>, Float32Array>>,
  Assert<Equal<ReturnType<Model['tokenVector']>, Float32Array | null>>,
  Assert<Equal<ReturnType<Model['similarity']>, number>>,
  Assert<Equal<ReturnType<Model['spanSimilarity']>, number>>,
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
