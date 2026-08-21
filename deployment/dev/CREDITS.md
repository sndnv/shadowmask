# Credits and third-party attribution

The Shadowmask dev deployment can optionally download real Creative Commons video clips as playable
fixtures. The scripts fetch them to your own machine on request; the clips themselves are not
redistributed in this repository. The clips remain under their own licenses, credited here.

Test fixtures under `crates/media/tests/fixtures` are a separate case: two of them are real-world
files that are redistributed here, credited in
[`../../crates/media/tests/fixtures/CREDITS.md`](../../crates/media/tests/fixtures/CREDITS.md).

## Video fixtures

### Big Buck Bunny (2008)

- Author: Blender Foundation
- Source: https://peach.blender.org (mirror: https://download.blender.org/peach/bigbuckbunny_movies/)
- License: Creative Commons Attribution 3.0 (CC BY 3.0), https://creativecommons.org/licenses/by/3.0/

### Elephants Dream (2006)

- Author: Blender Foundation / Netherlands Media Art Institute
- Source: https://www.elephantsdream.org (mirror: https://archive.org/details/ElephantsDream)
- License: Creative Commons Attribution 3.0 (CC BY 3.0), https://creativecommons.org/licenses/by/3.0/

## Local enrichment runtime and models

The optional local-inference features (transcription and translation) bundle the CTranslate2 runtime
(via the `ct2rs` crate) and run user-supplied model weights. The authoritative attribution (CTranslate2
and `ct2rs` under MIT, Whisper under MIT, Opus-MT under CC BY 4.0, and MADLAD-400 under Apache 2.0) and
the operator's licensing responsibilities live in
[`../ENRICHMENT.md`](../ENRICHMENT.md#licensing-and-attribution).
