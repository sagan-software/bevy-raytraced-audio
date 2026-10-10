# KEMAR headphone responses

`kemar.bin` contains the 368 stereo, diffuse-field-equalized impulse responses
from [MIT's KEMAR measurements](https://sound.media.mit.edu/resources/KEMAR.html),
by Bill Gardner and Keith Martin, MIT Media Laboratory, 1994.

Copyright 1994 MIT Media Laboratory. The authors provide the data free with no
restrictions on use, provided they are cited in research or commercial
applications. This attribution must accompany redistributions of this data.

Source: <https://sound.media.mit.edu/resources/KEMAR/diffuse.zip>.
Each record is a little-endian signed i16 elevation and azimuth in degrees,
followed by 128 interleaved stereo signed i16 samples at 44,100 Hz. Records are
sorted by (elevation, azimuth). Samples are unmodified; divide by 32768 to decode.
Azimuth 0 is forward, 90 right, 180 behind. Mirror azimuth and swap ears for the
left hemisphere. Elevation coverage is -40 to +90 degrees; directions below
-40 use the lowest measured elevation, not invented measurements.

These are generic mannequin responses, not personalized HRTFs. Localization
varies with the listener and headphones. Use plain stereo headphones without a
second spatializer for the intended result.

To reproduce the binary, download `diffuse.zip` from the source above and run
`python3 scripts/import-kemar.py /path/to/diffuse.zip` from the repository.
The importer verifies the WAV layout and SHA-256 of the resulting table.
