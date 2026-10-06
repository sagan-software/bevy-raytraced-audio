# Local research clones

Research snapshot: 2026-10-06. The research clones are kept outside this project under `$HOME/Code/github.com/` so they remain available for inspection without vendoring upstream code. Most are shallow clones; `template-bevy` retains its full history. The listed commit identifies the snapshot researched; recheck the upstream repository before implementation.

## Vercidium Audio organization

All 18 public repositories were cloned under `$HOME/Code/github.com/vercidium-audio/`. The repository inventory, purpose, and commit snapshots are in [the organization research](vercidium-audio.md).

The latest fetch found no `master` branch. The local clone is clean and follows `main` at `9bc21efeccdd1236e3e64cf7bb607823c60d4600`.

## Bevy and Rust audio references

| Local checkout | Commit snapshot | Role |
| --- | --- | --- |
| `$HOME/Code/github.com/bevyengine/bevy` | `b56fc29d3016e641754765244b5ba3f9cc504671`, Bevy 0.19.1 | Official Bevy source and examples; comparison tags include 0.17.3, 0.18.1, 0.19.1, and 0.20.0-rc.2 |
| `$HOME/Code/github.com/MaxenceMaire/audionimbus` | `5de93a7b6a9b40a65d8d30d8145f35d78c81691d` | Rust Steam Audio wrapper; maintained Bevy example |
| `$HOME/Code/github.com/MaxenceMaire/audionimbus-demo` | `b1ac8ab4d572286cff6d44e3c64be3dfc34eb4ed` | Archived Bevy audio propagation demo |
| `$HOME/Code/github.com/janhohenheim/bevy_steam_audio` | `bbcd1f220bdce5565b26ecdc01ba1d8c5f3ec777` | Work-in-progress Steam Audio Bevy integration |
| `$HOME/Code/github.com/corvusprudens/bevy_seedling` | `ab4a88b430d216e8614549926e3532bcec5ce2db` | Seedling 0.8.0 Bevy audio graph integration at this snapshot |
| `$HOME/Code/github.com/janhohenheim/bevy_seedling` | `5e11f13253754dd837a926db49f0cd33da2ca689` | Older 0.17 release-candidate fork retained as historical reference |
| `$HOME/Code/github.com/BillyDM/firewheel` | `34b35cad0535bf0a9542ac977b34744805d6cc03` | Rust audio graph engine and custom node APIs |
| `$HOME/Code/github.com/JustGoscha/omg-audio` | `7e10eae1d8498dc6a2c0dc46c68d2999ef2f7e9f` | Pure-Rust propagation and DSP implementation reference |
| `$HOME/Code/github.com/AudioGroupCologne/wavefront` | `73a1f2c79e66bbd332dfc77cfc425e776703dcd9` | 2D acoustic wave simulation reference outside ray tracing |
| `$HOME/Code/github.com/sagan-software/template-bevy` | `1f515985e4bd4cf63f0f52f92e3bb5b6353dbe13` | User's Nix-first Bevy starter; imported at `da85b65` and updated for the worktree cache helper |
| `$HOME/Code/github.com/sagan-software/dylints` | `0ed03375bd1190a44720a68f189fd9ca5fdf6710` | Current Sagan Dylint Quick Start and selected lint groups |

Both reference clones are clean and match their `origin/main` heads. The upstream repositories use `main`; neither has a `master` branch. The Sagan Dylints integration follows the current [Quick Start](../DYLINTS.md).

No upstream repository has been copied into this project. The repository contains the new Rust workspace, Bevy examples, Nix workflows, research notes, and the Markdown book.
