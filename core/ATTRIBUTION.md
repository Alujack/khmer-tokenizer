# Attribution

`khmer-tokenizer` is MIT/Apache-2.0, but the embedded dictionary
(`core/src/dict.txt`) is derived from third-party data. This file credits
that source per its license.

## Dictionary — `core/src/dict.txt`

Source: [`data/khmerdict.txt`](https://github.com/veer66/chamkho/blob/master/data/khmerdict.txt)
from [chamkho](https://github.com/veer66/chamkho) (Vee Satayamas / veer66).
chamkho ships that file under its own dedicated license file,
[`LICENSE-khmerdict`](https://github.com/veer66/chamkho/blob/master/LICENSE-khmerdict):

> The MIT License (MIT)
>
> Copyright (c) 2015 SIL NRSI
>
> Permission is hereby granted, free of charge, to any person obtaining a copy
> of this software and associated documentation files (the "Software"), to deal
> in the Software without restriction, including without limitation the rights
> to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
> copies of the Software, and to permit persons to whom the Software is
> furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in all
> copies or substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
> IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
> FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
> AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
> LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
> OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
> SOFTWARE.

`core/src/dict.txt` is produced from that file by `cargo xtask prepare-dict`
(`xtask/src/dict.rs`): trimmed, deduplicated, comments/blank lines dropped —
no words added, removed, or edited.

## Supplemental wordlist — `core/src/dict.supplement.txt`

A small **project-authored** wordlist (40 entries as of v0.3), licensed
MIT OR Apache-2.0 like the crate itself, loaded by
`KhmerTokenizer::with_default_dict()` alongside the base dictionary. It
covers modern vocabulary the 2015-era base list lacks: Cambodian province
names, countries, and transliterated loanwords (technology, health, news).

Provenance: candidate words were *ranked* by how often they appeared as
out-of-vocabulary tokens in public Khmer web-news text, then each entry was
confirmed absent from `dict.txt` and hand-verified for correct spelling
before inclusion. The list is original curation — no third-party wordlist
or corpus text is reproduced in it. Unlike `dict.txt`, it *is* meant to be
hand-edited: additions welcome via pull request.

## Not currently bundled

`khopilot/khmer-lexicon` (CC BY 4.0, 12,653 terms + word frequencies) was
evaluated as an alternative — see `docs/RESEARCH-2.md` §1e. It is **gated**
on HuggingFace (requires an authenticated, terms-accepted account) and ships
only as Parquet, so it isn't wired into the automated `prepare-dict` pipeline.
It remains a candidate bundleable frequency-table source if a Hugging Face
access token ever becomes available.

## Word frequencies (`Strategy::UnigramDp`) — not bundled, evaluation-only

`cargo xtask eval` counts word frequencies from khPOS's
`before-replace/train6.word` split (12,000 sentences, **CC BY-NC-SA 4.0**) to
benchmark `Strategy::UnigramDp` against the other strategies (see
`docs/BENCHMARKS.md`). This count table is computed **at eval time only** —
it is never written to a committed file, bundled in `core/src/dict.txt`, or
shipped in any binary. Production use of `UnigramDp` requires supplying your
own frequency table via `KhmerTokenizer::with_frequencies(...)`, sourced
under a license appropriate to your use case.

## HMM fallback (`HmmModel`, Phase 4 OOV handling) — not bundled, evaluation-only

`cargo xtask eval` also trains BMES (Begin/Middle/End/Single) tag counts from
the same khPOS `before-replace/train6.word` split (CC BY-NC-SA 4.0) to
benchmark `KhmerTokenizer::with_hmm(...)` (see `docs/BENCHMARKS.md` Phase 4).
Exactly the same posture as the frequency table above applies: these counts
are computed **at eval time only**, never written to a committed file,
bundled in the crate, or shipped in any binary. Production use of the HMM
fallback requires building your own model with
`HmmModel::from_counts(...)` from a segmented corpus you're licensed to use,
then attaching it via `KhmerTokenizer::with_hmm(...)`.
