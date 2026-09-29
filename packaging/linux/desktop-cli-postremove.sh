#!/bin/sh
set -eu

link=/usr/local/bin/msc
if [ -L "$link" ]; then
  target=$(readlink "$link")
  case "$target" in
    '/usr/lib/MSC 2/agent/msc'|/usr/lib/msc2-desktop-web/agent/msc)
      [ -e "$target" ] || rm "$link"
      ;;
  esac
fi
