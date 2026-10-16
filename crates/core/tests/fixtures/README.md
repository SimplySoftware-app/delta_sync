Tiny media fixtures for the file-signing tests (no secrets, no personal data).
All three are synthetic ffmpeg test patterns:

    ffmpeg -f lavfi -i testsrc=size=64x48:rate=10:duration=1 -frames:v 1 -update 1 -pix_fmt yuvj420p sample.jpg
    ffmpeg -f lavfi -i testsrc=size=64x48:rate=10:duration=1 -f lavfi -i sine=frequency=440:duration=1 \
      -c:v libx264 -pix_fmt yuv420p -c:a aac -shortest -movflags +faststart sample.mp4
    ffmpeg -i sample.mp4 -c copy -f mov sample.mov

`sample-transcoded.mp4` stands in for what the media processor renders from
`sample.mp4` (smaller, re-encoded, same faststart layout):

    ffmpeg -i sample.mp4 -vf scale=32:24 -c:v libx264 -crf 34 -pix_fmt yuv420p -c:a aac -b:a 24k \
      -movflags +faststart -fflags +bitexact -flags:v +bitexact -flags:a +bitexact sample-transcoded.mp4

The tests use the committed files, so they need no ffmpeg. `video.rs` also makes a
multi-megabyte clip with ffmpeg when it is installed (to cross a Merkle chunk
boundary) and skips that one test when it is not.
