#!/usr/bin/env bash
set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"
cd "$repository_root"

wasm_bindgen_version="0.2.129"
web_tools="$repository_root/target/web-tools"
target_dir="$(cargo metadata --no-deps --format-version 1 | jq -r '.target_directory')"
examples_package="bevy-raytraced-audio-examples"
example_names=(minimal_2d minimal_3d stress_2d stress_3d)
example_slugs=(minimal-2d minimal-3d stress-2d stress-3d)
build_id="${GITHUB_RUN_ID:-local-$(date +%s%N)}"
site_dir="$repository_root/target/pages-site-$build_id"

installed_version=""
if [[ -x "$web_tools/bin/wasm-bindgen" ]]; then
    installed_version="$("$web_tools/bin/wasm-bindgen" --version | awk '{print $2}')"
fi

if [[ "$installed_version" != "$wasm_bindgen_version" ]]; then
    cargo install --locked --version "$wasm_bindgen_version" --root "$web_tools" wasm-bindgen-cli
fi

export PATH="$web_tools/bin:$PATH"

# Compile each browser demo with WebGL2 and Bevy's WebAudio-backed audio plugin.
cargo build --locked --profile wasm-release --target wasm32-unknown-unknown \
    --package "$examples_package" --examples --features webgl2

# Keep each site build in its own ignored target directory.
mkdir -p "$site_dir/examples"
cp -a website/. "$site_dir/"

for index in "${!example_names[@]}"; do
    example_name="${example_names[$index]}"
    example_slug="${example_slugs[$index]}"
    example_dir="$site_dir/examples/$example_slug"
    wasm_file="$target_dir/wasm32-unknown-unknown/wasm-release/examples/$example_name.wasm"

    mkdir -p "$example_dir/pkg"
    cp -a assets "$example_dir/assets"
    wasm-bindgen --target web --out-name app --out-dir "$example_dir/pkg" "$wasm_file"
    gzip --best --no-name "$example_dir/pkg/app_bg.wasm"
    gzip --test "$example_dir/pkg/app_bg.wasm.gz"
done

# Include the Markdown book beside the runnable examples.
mdbook build docs --dest-dir "$site_dir/docs"

test -s "$site_dir/index.html"
test -s "$site_dir/docs/index.html"
for example_slug in "${example_slugs[@]}"; do
    test -s "$site_dir/examples/$example_slug/index.html"
    test -s "$site_dir/examples/$example_slug/pkg/app.js"
    test -s "$site_dir/examples/$example_slug/pkg/app_bg.wasm.gz"
    test -s "$site_dir/examples/$example_slug/assets/audio/bevy-raytraced-audio-chime.wav"
done

printf 'Built the gallery and %s browser examples in %s\n' "${#example_names[@]}" "$site_dir"
