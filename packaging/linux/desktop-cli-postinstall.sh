#!/bin/sh
set -eu

link=/usr/local/bin/msc
for candidate in '/usr/lib/MSC 2/agent/msc' /usr/lib/msc2-desktop-web/agent/msc; do
  if [ -x "$candidate" ]; then
    target=$candidate
    break
  fi
done
[ -n "${target:-}" ] || { echo 'MSC 2: packaged agent binary is missing' >&2; exit 1; }
[ -d /usr/local/bin ] || install -d -m 0755 /usr/local/bin
if [ -L "$link" ]; then
  current=$(readlink "$link")
  case "$current" in
    '/usr/lib/MSC 2/agent/msc'|/usr/lib/msc2-desktop-web/agent/msc) rm "$link" ;;
    *) echo "MSC 2: refusing to replace $link" >&2; exit 1 ;;
  esac
elif [ -e "$link" ]; then
  echo "MSC 2: refusing to replace $link" >&2
  exit 1
fi
ln -s "$target" "$link"
echo 'MSC 2: msc is available in /usr/local/bin; open a new shell if this shell does not include it on PATH.'
