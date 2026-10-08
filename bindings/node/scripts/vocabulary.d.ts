// SpaRs vocabulary types. Appended to the generated index.d.ts by scripts/declarations.mts;
// tools/test_node_vocabulary.py checks every list against its source.

/** spaCy's universal part-of-speech tags (`spacy.parts_of_speech`). */
export type UniversalPos = 'ADJ' | 'ADP' | 'ADV' | 'AUX' | 'CCONJ' | 'CONJ' | 'DET' | 'EOL' | 'INTJ' | 'NOUN' | 'NUM' | 'PART' | 'PRON' | 'PROPN' | 'PUNCT' | 'SCONJ' | 'SPACE' | 'SYM' | 'VERB' | 'X'

/** Entity IOB tags: B begins an entity, I continues it, O is outside any entity. */
export type EntityIob = 'B' | 'I' | 'O'

/**
 * Entity labels predicted by the supported official pipelines. Add your own labels for
 * autocomplete and type checking with declaration merging:
 *
 * ```typescript
 * declare module '@spars/node' {
 *   interface EntityLabels { PRODUCT_CODE: true }
 * }
 * ```
 */
export interface EntityLabels {
  CARDINAL: true; DATE: true; EVENT: true; FAC: true; GPE: true; LANGUAGE: true; LAW: true; LOC: true; MONEY: true
  NORP: true; ORDINAL: true; ORG: true; PERCENT: true; PERSON: true; PRODUCT: true; QUANTITY: true; TIME: true; WORK_OF_ART: true
}

/** Fine-grained tags predicted by the supported official pipelines. Extend it like {@link EntityLabels}. */
export interface FineGrainedTags {
  '$': true; "''": true; ',': true; '-LRB-': true; '-RRB-': true; '.': true; ':': true; ADD: true; AFX: true; CC: true
  CD: true; DT: true; EX: true; FW: true; HYPH: true; IN: true; JJ: true; JJR: true; JJS: true; LS: true; MD: true; NFP: true
  NN: true; NNP: true; NNPS: true; NNS: true; PDT: true; POS: true; PRP: true; 'PRP$': true; RB: true; RBR: true; RBS: true
  RP: true; SYM: true; TO: true; UH: true; VB: true; VBD: true; VBG: true; VBN: true; VBP: true; VBZ: true; WDT: true; WP: true
  'WP$': true; WRB: true; XX: true; _SP: true; '``': true
}

/** Dependency labels predicted by the supported official pipelines. Extend it like {@link EntityLabels}. */
export interface DependencyLabels {
  ROOT: true; acl: true; acomp: true; advcl: true; advmod: true; agent: true; amod: true; appos: true; attr: true; aux: true
  auxpass: true; case: true; cc: true; ccomp: true; compound: true; conj: true; csubj: true; csubjpass: true; dative: true
  dep: true; det: true; dobj: true; expl: true; intj: true; mark: true; meta: true; neg: true; nmod: true; npadvmod: true
  nsubj: true; nsubjpass: true; nummod: true; oprd: true; parataxis: true; pcomp: true; pobj: true; poss: true; preconj: true
  predet: true; prep: true; prt: true; punct: true; quantmod: true; relcl: true; xcomp: true
}

/** Official pipelines that `downloadModel` can install. */
export type OfficialModelName = 'en_core_web_sm' | 'en_core_web_md' | 'en_core_web_lg'

/** Model names for `loadModel` and `ModelStore`. Register custom installations like {@link EntityLabels}. */
export interface ModelNames {
  en_core_web_sm: true; en_core_web_md: true; en_core_web_lg: true
}

/**
 * Label sets that accept only registered values. By default each set also accepts any other
 * string, because models and documents can carry labels the types do not know. Opt in per set:
 *
 * ```typescript
 * declare module '@spars/node' {
 *   interface StrictLabelSets { entity: true }
 * }
 * ```
 */
export interface StrictLabelSets {}

type RegisteredLabel<Registry> = Extract<keyof Registry, string>
type LabelSet<Registry, Name extends string> = StrictLabelSets extends { [K in Name]: true }
  ? RegisteredLabel<Registry>
  : RegisteredLabel<Registry> | (string & {})

/** A registered entity label, or any string unless `StrictLabelSets` has `entity`. */
export type EntityLabel = LabelSet<EntityLabels, 'entity'>
/** A registered fine-grained tag, or any string unless `StrictLabelSets` has `tag`. */
export type FineGrainedTag = LabelSet<FineGrainedTags, 'tag'>
/** A registered dependency label, or any string unless `StrictLabelSets` has `dependency`. */
export type DependencyLabel = LabelSet<DependencyLabels, 'dependency'>
/** A registered model name, or any other name or directory path. */
export type ModelName = RegisteredLabel<ModelNames> | (string & {})

/** PhraseMatcher attributes, spelled as spaCy spells them. */
export type PhraseAttribute = 'ORTH' | 'TEXT' | 'LOWER' | 'NORM' | 'LEMMA' | 'POS' | 'TAG' | 'DEP' | 'MORPH'
  | 'IS_ALPHA' | 'IS_ASCII' | 'IS_DIGIT' | 'IS_LOWER' | 'IS_UPPER' | 'IS_TITLE' | 'IS_PUNCT' | 'IS_SPACE' | 'IS_BRACKET'
  | 'IS_QUOTE' | 'IS_LEFT_PUNCT' | 'IS_RIGHT_PUNCT' | 'IS_CURRENCY' | 'IS_STOP' | 'LIKE_NUM' | 'LIKE_URL' | 'LIKE_EMAIL' | 'LENGTH'

/** Token attributes compared as strings. */
export type StringAttribute = 'text' | 'lower' | 'norm' | 'lemma'
/** Lexical flag attributes; they need a model lexicon and the `flag` predicate. */
export type FlagAttribute = 'is_alpha' | 'is_digit' | 'is_space' | 'is_punct' | 'like_num' | 'is_lower' | 'is_upper'
  | 'is_title' | 'is_ascii' | 'is_currency' | 'is_stop' | 'is_bracket' | 'is_quote' | 'is_left_punct' | 'is_right_punct'
  | 'like_url' | 'like_email'
/** Every token attribute a TokenMatcher or DependencyMatcher condition can compare. */
export type TokenAttribute = StringAttribute | 'pos' | 'tag' | 'dep' | 'morphology' | FlagAttribute | 'length'

/** Numeric comparison operators for `length`. */
export type Comparison = '==' | '!=' | '>=' | '<=' | '>' | '<'
/** String set predicates; `is_subset` and `intersects` on a single value behave like `in`. */
export type StringSetKind = 'in' | 'not_in' | 'is_subset' | 'is_superset' | 'intersects'
/** Integer set predicates for `length`. */
export type IntegerSetKind = 'in_integers' | 'not_in_integers' | 'is_subset_integers' | 'is_superset_integers' | 'intersects_integers'

/** A string predicate whose values autocomplete from `Value`. */
export type StringPredicate<Value extends string = string> =
  | { kind: 'equals'; value: Value }
  | { kind: StringSetKind; values: Array<Value> }

/** One condition on a token. The attribute decides which predicates and values are valid. */
export type TokenConstraint =
  | { attribute: StringAttribute; predicate: StringPredicate }
  | { attribute: 'pos'; predicate: StringPredicate<UniversalPos | ''> }
  | { attribute: 'tag'; predicate: StringPredicate<FineGrainedTag | ''> }
  | { attribute: 'dep'; predicate: StringPredicate<DependencyLabel | ''> }
  | { attribute: 'morphology'; predicate: StringPredicate | { kind: 'morph_superset' | 'morph_intersects'; values: Array<string> } }
  | { attribute: FlagAttribute; predicate: { kind: 'flag'; value: boolean } }
  | { attribute: 'length'; predicate: { kind: 'compare'; operator: Comparison; value: number } | { kind: IntegerSetKind; values: Array<number> } }

/** How often a token pattern item may match. Only `range` takes bounds; `max` defaults to unbounded. */
export type TokenRepetition =
  | { kind: 'once' | 'optional' | 'zero_or_more' | 'one_or_more' | 'negated' }
  | { kind: 'range'; min: number; max?: number }

/** A spaCy `DependencyMatcher` relation operator, read as "left OPERATOR right"; see `DependencyLink.relation`. */
export type DependencyRelation = '<' | '>' | '<<' | '>>' | '.' | '.*' | ';' | ';*' | '$+' | '$-' | '$++' | '$--'
  | '>+' | '>-' | '>++' | '>--' | '<+' | '<-' | '<++' | '<--'

/**
 * Codes on SpaRs errors; `isSparsError` recognises them. Argument conversion failures in the native
 * layer carry Node-API status codes such as `InvalidArg` instead, and argument checks in JavaScript
 * throw a plain `TypeError` or `RangeError`.
 */
export type SparsErrorCode = 'SPARS_IO' | 'SPARS_INVALID_MODEL' | 'SPARS_UNSUPPORTED' | 'SPARS_INVALID_TEXT' | 'SPARS_TEXT_TOO_LONG'
  | 'SPARS_BOUNDS' | 'SPARS_INFERENCE' | 'SPARS_INVALID_PATTERN' | 'SPARS_BUSY' | 'SPARS_INPUT_LIMIT' | 'SPARS_NATIVE_INCOMPATIBLE'

/** An error raised by SpaRs, distinguished by its `code`. */
export interface SparsError extends Error {
  code: SparsErrorCode
}
