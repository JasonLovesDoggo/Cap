# Cap performance demos

These are normal-speed recordings of the real GPUI desktop app, captured with Cap's local CLI. Cap's current MCP exposes cloud/library tools but no local recording API.

PR7: audition four synthetic ten-minute stereo tracks A, B, C and D, then undo to C. The adjacent panel reads the actual Cap process RSS once per second. Capture overhead is present; the PR benchmark table comes from separate quiet runs. These videos are silent.

| Before | After |
| --- | --- |
| [Watch baseline](pr7-before.mp4) | [Watch revised cache](pr7-after.mp4) |

Baseline source: 1fabfaefa84ed4bcc98eec1c861ca73a7ba51dd9. Recorded candidate source: 2067b4a50be8565167c2355f17e1c586b47915c0; published as 9d19532a9d28fedf46e8816d1834ca646f774ba5 with identical complete source tree 0793f172bbbfe57456e3f236e77e987d247a038b.

Each video preserves all 1,060 frames at 1600×1000, 30 fps, 35.333333 seconds. Cap-exported originals were recompressed for delivery with H.264 CRF28/medium/faststart; no trimming, resizing or speed changes. Original Cap projects and exports are retained in the experiment workspace.

PR6: actual camera Off → Light Blur → Heavy Blur playback, followed by a Heavy Blur export. Same baseline main as above; candidate 9d32c6483f6450ce04d3b37196e4d2e7f96c809e. The source portrait is the public-domain NASA Eileen Collins image distributed as scikit-image's astronaut sample. Native AMD media renderer; CPU ONNX segmentation; Xvfb frontend. These are correctness demonstrations, not timing comparisons.

| Before | After |
| --- | --- |
| [Baseline MP4](pr6-before.mp4) | [Camera bounds MP4](pr6-after.mp4) |

PR6 videos preserve all captured frames at 1600×1000 and 30 fps: before 662 frames / 22.066667 seconds; after 670 frames / 22.333333 seconds. Same CRF28 web-delivery encoding, with no trimming or speed changes.
