Tiny media fixtures for the file-signing tests (no secrets, no personal data).
All three are synthetic ffmpeg test patterns:

    ffmpeg -f lavfi -i testsrc=size=64x48:rate=10:duration=1 -frames:v 1 -update 1 -pix_fmt yuvj420p sample.jpg
    ffmpeg -f lavfi -i testsrc=size=64x48:rate=10:duration=1 -f lavfi -i sine=frequency=440:duration=1 \
      -c:v libx264 -pix_fmt yuv420p -c:a aac -shortest -movflags +faststart sample.mp4
    ffmpeg -i sample.mp4 -c copy -f mov sample.mov
