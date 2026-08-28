#!/usr/bin/env bash
# 推送规则：以最新版本为基准增量 bump patch → 同步版本 → 提交并推送 → 打 tag 并推送。
# 不做任何打包（cargo package / docker build 一律跳过）。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

latest_tag=$(git tag --sort=-v:refname | head -1 || true)
if [[ -z "$latest_tag" ]]; then
  echo "!! 没有现存 tag，无法增量 bump。首次发布请手动指定版本。" >&2
  exit 1
fi

current=$(grep -m1 '^version = ' Cargo.toml | grep -oP '\d+\.\d+\.\d+')
tag_ver=${latest_tag#v}

# 以 Cargo.toml 与最新 tag 的较大者作为 bump 基准
base=$(printf '%s\n%s\n' "$current" "$tag_ver" | sort -V | tail -1)
new_ver=$(echo "$base" | awk -F. '{printf "%d.%d.%d", $1, $2, $3+1}')
new_tag="v$new_ver"

if [[ "$new_tag" == "$latest_tag" ]]; then
  echo "!! 版本未变化（$latest_tag），中止。" >&2
  exit 1
fi

echo "== 最新 tag: $latest_tag | 当前版本: $current | 新版本: $new_ver"

# 同步 workspace 版本
sed -i "s/^version = \"$base\"/version = \"$new_ver\"/" Cargo.toml

# 同步 Cargo.lock（cargo 会更新 workspace 成员版本）
cargo check --workspace --quiet

git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "v$new_ver: 版本同步 $new_ver"
git push

git tag "$new_tag"
git push origin "$new_tag"

echo "== 完成：$new_tag 已推送（未打包）"
