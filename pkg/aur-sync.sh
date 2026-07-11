#!/bin/sh
set -eu
repo=$(cd "$(dirname "$0")/.." && pwd)
ver="0.0.r$(git -C "$repo" rev-list --count HEAD).$(git -C "$repo" rev-parse --short HEAD)"
for d in "$HOME/aur/recipe" "$HOME/aur/recipe-git"; do
      [ -d "$d/.git" ] || { echo "aur-sync: missing $d" >&2; continue; }
      sed -i "s/^pkgver=.*/pkgver=$ver/" "$d/PKGBUILD"
      (cd "$d" && makepkg --printsrcinfo > .SRCINFO)
      git -C "$d" diff --quiet -- PKGBUILD .SRCINFO && { echo "aur-sync: $(basename "$d") already at $ver" >&2; continue; }
      git -C "$d" add PKGBUILD .SRCINFO
      git -C "$d" commit -q -m "bump to $ver"
      timeout 60 git -C "$d" push -q origin master
      echo "aur-sync: pushed $(basename "$d") $ver" >&2
done
