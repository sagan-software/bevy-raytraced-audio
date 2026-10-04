# Local research clones

Research snapshot: 2026-10-04. These shallow clones are kept outside this project under `/home/sagan/Code/github.com/` so they remain available for inspection without vendoring upstream code. The listed commit identifies the snapshot researched; recheck the upstream repository before implementation.

## Vercidium Audio organization

All 18 public repositories were cloned under `/home/sagan/Code/github.com/vercidium-audio/`. The repository inventory, purpose, and commit snapshots are in [the organization research](vercidium-audio.md).

## Bevy and Rust audio references

| Local checkout | Commit snapshot | Role |
| --- | --- | --- |
| `/home/sagan/Code/github.com/bevyengine/bevy` | `b56fc29d3016e641754765244b5ba3f9cc504671`, Bevy 0.19.1 | Official source and examples. Tags for 0.17.3, 0.18.1, 0.19.1, and 0.20.0-rc.2 were also fetched for comparison. |
| `/home/sagan/Code/github.com/MaxenceMaire/audionimbus` | `5de93a7b6a9b40a65d8d30d8145f35d78c81691d` | Rust Steam Audio wrapper and maintained Bevy example. |
| `/home/sagan/Code/github.com/MaxenceMaire/audionimbus-demo` | `b1ac8ab4d572286cff6d44e3c64be3dfc34eb4ed` | Archived Bevy audio propagation demo. |
| `/home/sagan/Code/github.com/janhohenheim/bevy_steam_audio` | `bbcd1f220bdce5565b26ecdc01ba1d8c5f3ec777` | Work-in-progress Steam Audio Bevy integration. |
| `/home/sagan/Code/github.com/corvusprudens/bevy_seedling` | `ab4a88b430d216e8614549926e3532bcec5ce2db` | Current Seedling 0.8.0 Bevy audio graph integration at the snapshot. |
| `/home/sagan/Code/github.com/janhohenheim/bevy_seedling` | `5e11f13253754dd837a926db49f0cd33da2ca689` | Older 0.17 release-candidate fork; retained only as historical reference. |
| `/home/sagan/Code/github.com/BillyDM/firewheel` | `34b35cad0535bf0a9542ac977b34744805d6cc03` | Rust audio graph engine and custom node APIs. |
| `/home/sagan/Code/github.com/JustGoscha/omg-audio` | `7e10eae1d8498dc6a2c0dc46c68d2999ef2f7e9f` | Closest pure-Rust propagation and DSP implementation reference. |
| `/home/sagan/Code/github.com/AudioGroupCologne/wavefront` | `73a1f2c79e66bbd332dfc77cfc425e776703dcd9` | 2D acoustic wave simulation reference, not ray tracing. |

The local Sagan Dylints checkout is `/home/sagan/Code/github.com/sagan-software/dylints`. It has pre-existing uncommitted user work and was not changed. The reproducible Dylints pin and integration instructions are documented in [Dylints integration](../DYLINTS.md).

No upstream repository has been copied into this project. The current project repository contains Markdown planning and research files only.
