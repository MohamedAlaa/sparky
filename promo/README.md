# Sparky promo video

- [`docs/media/sparky-promo.mp4`](../docs/media/sparky-promo.mp4): 36 s, 1920×1080, 60 fps, H.264 + AAC (~14 MB)
- The ~120 MB master (CRF 17) is too big for git; rebuild it with the steps below.

Everything is generated: real app motion, compositing and music.

1. **Record the app** (real UI motion, demo mailbox only, never your mail).
   `src/recorder.rs` plays scripted input with simulated time at 60 fps:

       cd ../app && cargo build --release
       for s in nav palette search swipe accent shortcuts reply; do
         SPARKY_DATA_DIR=$(mktemp -d) ./target/release/sparky --record rec/$s $s
       done

2. **Soundtrack** (120 BPM, synthesized, hits synced to the cut):

       swiftc -O source/audio.swift -o audio && ./audio soundtrack.wav

3. **Composite and encode** (3D camera, kinetic type, keycaps, particles):

       swiftc -O source/render.swift -o render
       ./render rec ../app/assets/Sparky.png | ffmpeg -f rawvideo -pix_fmt bgra \
         -s 1920x1080 -r 60 -i - -i soundtrack.wav -c:v libx264 -crf 17 \
         -pix_fmt yuv420p -c:a aac -b:a 256k -shortest Sparky-promo.mp4

   `./render rec ../app/assets/Sparky.png <frame>` writes one preview PNG.
