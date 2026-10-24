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

Additional current-main demonstrations:

| PR / workflow | Before | After | Scope |
| --- | --- | --- | --- |
| #3 same-source transition and backward replay | [MP4](pr3-before.mp4) | [MP4](pr3-after.mp4) | 720p correctness demonstration; 4K readiness remains unresolved |
| #4 software export | [MP4](pr4-before.mp4) | [MP4](pr4-after.mp4) | Correct output; historical gain is small |
| #5 style boundary, disable and undo | [MP4](pr5-before.mp4) | [MP4](pr5-after.mp4) | Correctness demonstration; feature gain remains unproven |
| #8 eight-clip export | [MP4](pr8-before.mp4) | [MP4](pr8-after.mp4) | Live current RSS; quiet benchmarks measure peak, not post-export retention |
| #9 selected-source estimates and export | [MP4](pr9-before.mp4) | [MP4](pr9-after.mp4) | Live current RSS; benchmark timings collected separately |

All recordings use Cap's real local recorder, with no microphone/system-audio capture. Web delivery preserves every frame, original duration, 1600×1000 and 30 fps; CRF28 compression only, with no trimming or speed changes. They show the actual process RSS rather than invented counters. Recording overhead means the videos are not the performance samples.

Recorded/reviewed source identities: #3 local053f89b7 published1da7cbfc, identical tree dd30e22586eca416ad05a9056984a65d6fd1d1bc; #4 d9c89cf1806744beee965575bbdc9cbaa979c4a6; #5 localc06d6029 published1c133e2c, identical tree c2c6fa96e1bda0fac1cb5a6bb3b2b987d7c5d4eb; #8 73cdf394c2fa80162a57b458282a35ba7afc4137; #9 local88ccb160 published5ff41b9c, identical tree ae4a12a75c1aa17c828439c34d86935560854047. All before builds use main1fabfaefa84ed4bcc98eec1c861ca73a7ba51dd9.

Deferred editor source initialization: open an eight-source project whose timeline uses source 7, seek, return to frame zero, and play. The panel samples actual Cap process RSS. Both videos preserve 419 frames at 1600×1000, 30 fps, 13.966667 seconds. They are demonstrations with capture overhead, separate from the quiet measurements.

| Before | After |
| --- | --- |
| [Baseline MP4](deferred-editor-before.mp4) | [Deferred editor sources MP4](deferred-editor-after.mp4) |

Recorded candidate source: 5ec43bdeb4b2853ac9a7b5cc4f8543c46791b2d1; published as bcdd091bf696c25be499a26ec01a8a91dae9b33d with identical complete source tree 5a75b28f898356d44c35437491f7941364af205e. The baseline is main 1fabfaefa84ed4bcc98eec1c861ca73a7ba51dd9. No microphone or system audio was captured; the Cap exports contain synthesized audio verified as exact silence. Original Cap projects and exports are retained. Delivery uses the same CRF28 encoding without trimming, resizing or speed changes.
