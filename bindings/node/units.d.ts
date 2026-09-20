/** Distinct units prevent accidental interchange in TypeScript. Values are numbers at runtime. */
export type ByteOffset = number & { readonly __byteOffset: unique symbol };
export type CodePointOffset = number & { readonly __codePointOffset: unique symbol };
export type Utf16Offset = number & { readonly __utf16Offset: unique symbol };
export type TokenIndex = number & { readonly __tokenIndex: unique symbol };
