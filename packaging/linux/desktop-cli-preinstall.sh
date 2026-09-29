#!/bin/sh
set -eu

link=/usr/local/bin/msc
if [ -L "$link" ]; then
  target=$(readlink "$link")
  case "$target" in
    '/usr/lib/MSC 2/agent/msc'|/usr/lib/msc2-desktop-web/agent/msc) exit 0 ;;
  esac
elif [ ! -e "$link" ]; then
  exit 0
fi
echo "MSC 2: $link belongs to another installation; remove that command before installing the desktop package" >&2
exit 1
