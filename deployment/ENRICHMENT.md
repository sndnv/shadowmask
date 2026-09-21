# Local AI enrichment

Shadowmask can generate subtitles and upscale video locally, with no external service, using the
optional **enrichment** build. Everything here is off by default and admin-configurable.

## What it does

- **Transcription** (`transcription`): speech to subtitles with a Whisper model (CTranslate2 runtime).
- **Translation** (`translation`): machine translation of subtitles with an Opus-MT or MADLAD model.
- **Upscaling** (`upscaling`): resolution upscaling. This is pure FFmpeg and needs no model.

Transcription and translation run on the CPU. They are compiled into the server only in the
enrichment image; the base image cannot run them regardless of configuration. Upscaling is in every
image, as is GPU-accelerated transcoding ([hardware acceleration](README.md#hardware-acceleration)).

## The enrichment image

The enrichment features are a separate build stage, published as its own tag:

```
ghcr.io/sndnv/shadowmask/server:<version>-enrichment
ghcr.io/sndnv/shadowmask/server:latest-enrichment
```

It is `FROM` the base image plus the bundled CTranslate2 runtime, so it is larger and slower to
build. Point your compose `image:` at the `-enrichment` tag (dev's `docker-compose.yml` already
targets `runtime-enrichment`; production defaults to the base image, so switch the tag when you
want AI).

## Enabling features

Each feature is a single environment variable, off unless set to `true`:

```
SHADOWMASK_ENRICHMENT_TRANSCRIPTION_ENABLED=true
SHADOWMASK_ENRICHMENT_TRANSLATION_ENABLED=true
SHADOWMASK_ENRICHMENT_UPSCALING_ENABLED=true
```

Transcription and translation additionally need a model on disk (below); without one the server
still starts and those jobs fail with a backend error until a model is present. Upscaling works with
no model.

The `provider` setting (`SHADOWMASK_ENRICHMENT_TRANSCRIPTION_PROVIDER`, etc.) is currently
informational only: when a feature is enabled the bundled local provider is what runs. External and
combined providers are not implemented yet, so leave `provider` at its default.

## Models

Models must be in **CTranslate2 format**, not raw Hugging Face transformers checkpoints. Mount a
directory into the container and set `SHADOWMASK_ENRICHMENT_MODEL_CACHE` to it. The server looks for
two subdirectories: `<cache>/transcription` and `<cache>/translation`. Each is a **collection of
candidate model folders**, not a single model: put as many model folders in as you like. On startup
the server scans them in alphanumeric order and uses the first folder that holds a valid model,
logging a warning for every folder it rejects and an info line naming the model it picked.

If a feature is enabled but its directory holds folders and none of them is a valid model, the
server **fails to start**. The admin clearly meant enrichment to work, so the misconfiguration is
made loud rather than silently skipped. An empty or absent directory is not fatal: the feature
reports `available: true, enabled: false` until a model is added.

A folder is a valid model when it contains:

- transcription: `model.bin`, `config.json`, `tokenizer.json`, `preprocessor_config.json`
- translation: `model.bin`, `config.json`, and a tokenizer the runtime can read, which is
  `tokenizer.json`, or `source.spm` and `target.spm`, or `vocab.json` and `merges.txt`

Override either base directory with `SHADOWMASK_ENRICHMENT_TRANSCRIPTION_MODEL_PATH` /
`SHADOWMASK_ENRICHMENT_TRANSLATION_MODEL_PATH`; these name the directory that is scanned, not a
single model.

Recommended layout, mounted read-only with `MODEL_CACHE=/models`:

```
models/
  transcription/
    faster-whisper-large-v3/   # one or more Whisper model folders
  translation/
    opus-mt-en-es/             # one or more translation model folders
```

```
volumes:
  - ./models:/models:ro
environment:
  SHADOWMASK_ENRICHMENT_MODEL_CACHE: "/models"
```

### Transcription (Whisper)

The `faster-whisper` models on Hugging Face are already in CTranslate2 format, so a direct download
is enough. Put the model folder under `transcription/`. Larger models are more accurate and slower.
Prefer an `int8` build for the CPU-only image, for the same reason as translation (see the
`--quantization int8` note below): a float16 model is promoted to float32 on load and takes about
twice the RAM. If only a float16 build is available, re-quantize it with
`ct2-transformers-converter --model <hf-whisper-model> --output_dir <dir> --quantization int8`.

### Translation

A translation model is driven in one of two ways:

- **Single-direction models** (for example `Helsinki-NLP/opus-mt-en-es`, English to Spanish) need no
  extra configuration, but only translate that one pair. Fine when you have a single target language.
- **Multilingual models** (MADLAD, Opus-MT `en-mul`, and similar) translate into whichever language
  you ask for, so one model covers every entry in `SHADOWMASK_TARGET_LANGUAGES`. They need a language
  token, which you set with a prefix (below).

A prefix is a template. `{target}` is replaced with the target language code (from
`SHADOWMASK_TARGET_LANGUAGES`, for example `es`) and `{source}` with the source code (empty when
unknown). `SHADOWMASK_ENRICHMENT_TRANSLATION_SOURCE_PREFIX` is prepended to the input text;
`SHADOWMASK_ENRICHMENT_TRANSLATION_TARGET_PREFIX` is fed to the decoder as its first token. Set
whichever your model expects:

| Model family          | Prefix setting                                     |
| --------------------- | -------------------------------------------------- |
| MADLAD                | `SHADOWMASK_ENRICHMENT_TRANSLATION_SOURCE_PREFIX=<2{target}>`  |
| Opus-MT multilingual  | `SHADOWMASK_ENRICHMENT_TRANSLATION_SOURCE_PREFIX=>>{target}<<` |
| decoder-token models  | `SHADOWMASK_ENRICHMENT_TRANSLATION_TARGET_PREFIX={target}`     |

Models are published as transformers checkpoints, so a raw download is not enough. Convert with
`ct2-transformers-converter` from the `ctranslate2` pip package. Two flags matter:

- **`--quantization int8`** — the enrichment image runs on CPU (no CUDA), where float16 is not
  computed efficiently. Left as float16, CTranslate2 promotes the model to float32 on load (logging
  `compute type ... float16 ... converted to ... float32`), which roughly doubles its memory. Convert
  to `int8` up front and the model is smaller on disk, uses far less RAM, runs faster on CPU, and the
  warning goes away.
- **`--copy_files <tokenizer>`** — bring the tokenizer along, or the folder is rejected as having no
  tokenizer.

**Convert with `transformers` 4.x, not 5.x.** transformers 5.x mishandles T5-family models that keep
an untied output projection (MADLAD-400 is one). It forces `tie_word_embeddings=True` and ties the
encoder input embedding onto the `lm_head` weights, so the converted model ignores the source text
and emits fluent but unrelated output, or a single token repeated. transformers 4.x (for example
4.57) respects the model's own `tie_word_embeddings: false` and converts correctly. Only the
conversion environment needs 4.x; the runtime is unaffected. A quick check: after loading the source
model, `encoder.embed_tokens.weight` must equal `decoder.embed_tokens.weight`, not `lm_head.weight`.

```
pip install ctranslate2 "transformers<5" sentencepiece

# single-direction Opus-MT (English to Spanish), no prefix needed
ct2-transformers-converter --model Helsinki-NLP/opus-mt-en-es \
  --output_dir models/translation/opus-mt-en-es \
  --quantization int8 --copy_files source.spm target.spm vocab.json

# multilingual MADLAD, pair with SOURCE_PREFIX=<2{target}>
ct2-transformers-converter --model google/madlad400-3b-mt \
  --output_dir models/translation/madlad400-3b-mt \
  --quantization int8 --copy_files tokenizer.json
```

If you already converted a model without its tokenizer, you do not have to redo the conversion: copy
the tokenizer the model ships (`tokenizer.json`, or the SentencePiece model as `source.spm` and
`target.spm`) into the output folder. A valid folder needs `model.bin`, `config.json`, and one of
those tokenizer sets (see the file list under [Models](#models)).

## Verifying

`GET /server/info` reports a `capabilities` array. Each entry has `available` (compiled in) and
`enabled` (configured on and, for transcription/translation, a usable model):

```json
{ "name": "transcription", "available": true, "enabled": true }
```

A capability that is `available: false` means the running image was not built with it (use the
`-enrichment` tag). `available: true, enabled: false` means it is compiled but turned off or missing
a model.

## Concurrency

Enrichment jobs (transcription, translation, upscaling) run on a **separate queue** from everything
else, capped at `SHADOWMASK_ENRICHMENT_CONCURRENCY` (default `1`). This bounds how many models are
resident at once, which is the main defence against out-of-memory kills: a single large model can
easily need many gigabytes, and loading several at once is a common way to get the container killed.
The cap is intentionally low; raise it only if you have the RAM (and CPU) for concurrent model runs.

The two queues are independent, so a slow model job never blocks ordinary work (library scans,
metadata, artwork, and the like), which keep their own `SHADOWMASK_WORKER_CONCURRENCY` limit.

## Operational notes

- Transcription and translation are CPU-bound and can be slow on large media; they run as background
  jobs and never block playback.
- Enabling a feature with an empty model directory does not crash the server; it reports
  `enabled: false` until a model is added. Enabling it with candidate folders that are all invalid
  is fatal at startup (see Models).
- At startup the server compares each selected model's on-disk size against the memory it is allowed
  to use (the container's cgroup limit when containerized, otherwise host available memory) and logs
  a warning when an out-of-memory kill looks likely. If it cannot read that limit it warns anyway,
  so a silent misconfiguration still surfaces.
- Model weights are downloaded by you and are not part of any Shadowmask image.

## Licensing and attribution

Shadowmask is licensed under Apache-2.0 (see the repository `LICENSE`). The enrichment image and the
model weights you supply carry their own licenses.

### Bundled in the enrichment image

The `ct2rs` and `sentencepiece-sys` crates compile vendored C++ source into the `-enrichment`
binary. `cargo-about` cannot see that source, so `THIRD-PARTY-LICENSES.md` credits only the crates
themselves.

Every `LICENSE*` and `COPYING*` file in those two crate sources is therefore swept into
[`licenses/enrichment/`](../licenses/enrichment) and copied into the image at
`/usr/share/doc/shadowmask/third-party-licenses/`. Paths are preserved, so each text names the
library it came from — `ct2rs/CTranslate2/third_party/ruy/LICENSE`, and so on.

Refresh the sweep with `python3 licenses/refresh_licenses.py`; `qa.py`'s `vendored` step fails when
it has drifted from the locked crate versions. A new dependency that vendors C/C++ source must be
added to that script's `CRATES` list — the `-sys` suffix is the usual tell.

Only the `-enrichment` image bundles this.

What the enrichment image inherits from the base one is covered in
[`SERVER-IMAGE-CREDITS.md`](./SERVER-IMAGE-CREDITS.md).

### Model weights (admin-supplied, not distributed by Shadowmask)

Shadowmask ships no model weights. You download them and are responsible for complying with the
license of whichever model you choose. Common choices:

- **Whisper** (OpenAI) - MIT license - https://github.com/openai/whisper
- **Opus-MT** (Helsinki-NLP) - CC-BY-4.0 - https://github.com/Helsinki-NLP/Opus-MT
- **MADLAD-400** (Google) - Apache-2.0 - https://huggingface.co/google/madlad400-3b-mt

Attribution and share-alike terms (CC-BY-4.0, for example) are yours to satisfy as the operator.
