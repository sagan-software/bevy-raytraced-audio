#!/usr/bin/env python3
"""Reproduce the embedded, unmodified MIT diffuse-field KEMAR response table.

Download https://sound.media.mit.edu/resources/KEMAR/diffuse.zip separately,
then pass that archive as the only argument. See src/hrtf/README.md for credits.
"""

import hashlib
import io
from pathlib import Path
import re
import struct
import sys
import wave
import zipfile


def main() -> None:
    """Validate the upstream WAV layout, pack records, and verify the resulting bytes."""
    with zipfile.ZipFile(sys.argv[1]) as archive:
        records = []
        for name in archive.namelist():
            match = re.search(r"H(-?\d+)e(\d+)a\.wav$", name)
            if match:
                records.append((int(match[1]), int(match[2]), name))
        output = bytearray()
        for elevation, azimuth, name in sorted(records):
            with wave.open(io.BytesIO(archive.read(name))) as source:
                layout = (source.getnchannels(), source.getsampwidth(),
                          source.getframerate(), source.getnframes())
                if layout != (2, 2, 44100, 128):
                    raise ValueError(f"Unexpected KEMAR layout: {name}: {layout}")
                output += struct.pack("<hh", elevation, azimuth)
                output += source.readframes(128)
    digest = hashlib.sha256(output).hexdigest()
    if digest != "e602d2754afdafcc56a8ac3c174449d39a892a72843625013919b93e62b2f287":
        raise ValueError(f"Unexpected measurement bank checksum: {digest}")
    destination = Path(__file__).resolve().parents[1] / "src/hrtf/kemar.bin"
    destination.write_bytes(output)
    print(f"Wrote {len(records)} records ({len(output)} bytes) to {destination}")


if __name__ == "__main__":
    main()
