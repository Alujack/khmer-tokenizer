/* tslint:disable */
/* eslint-disable */

export class KhmerTokenizer {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Whether `word` is in this tokenizer's dictionary.
     */
    contains(word: string): boolean;
    /**
     * `new KhmerTokenizer(options?)`
     *
     * Options (all optional, matching the Python constructor):
     * - `words: string[]` — custom word list instead of the embedded
     *   default dictionary
     * - `strategy: "minwords" | "fmm" | "bimm" | "unigram" | "tagger"` —
     *   segmentation strategy (default "minwords")
     * - `frequencies: Record<string, number>` — word counts for `"unigram"`
     * - `normalization: boolean` — orthographic normalization (default true)
     * - `oovGrouping: boolean` — group unmatched cluster runs into one
     *   unknown-word token each (default true)
     */
    constructor(options?: object | null);
    /**
     * Segment Khmer text into words. Returns an array of strings.
     */
    segment(text: string): string[];
    /**
     * Number of words in the dictionary (like `Map#size`).
     */
    readonly size: number;
}

/**
 * Whether a single character (one code point) is in the Khmer block.
 */
export function isKhmer(c: string): boolean;

/**
 * Repair common orthographic corruptions (wrong mark/COENG ordering,
 * including the damage Unicode NFC itself inflicts on Khmer).
 */
export function normalize(text: string): string;

/**
 * Fully normalize text: performs combining character ordering, orthographic
 * replacements, common spelling corrections, and punctuation/whitespace cleanup.
 */
export function normalizeFull(text: string): string;

/**
 * Split text into Khmer Character Clusters (orthographic syllables).
 */
export function splitKcc(text: string): string[];

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_khmertokenizer_free: (a: number, b: number) => void;
    readonly isKhmer: (a: number) => number;
    readonly khmertokenizer_contains: (a: number, b: number, c: number) => number;
    readonly khmertokenizer_new: (a: number) => [number, number, number];
    readonly khmertokenizer_segment: (a: number, b: number, c: number) => [number, number];
    readonly khmertokenizer_size: (a: number) => number;
    readonly normalize: (a: number, b: number) => [number, number];
    readonly normalizeFull: (a: number, b: number) => [number, number];
    readonly splitKcc: (a: number, b: number) => [number, number];
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __externref_drop_slice: (a: number, b: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
