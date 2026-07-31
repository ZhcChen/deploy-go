#!/bin/sh
set -eu

case " $* " in
  *" -t ed25519 "*) ;;
  *) echo "missing ed25519 restriction" >&2; exit 2 ;;
esac

printf '%s\n' '[node.example.test]:22 ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAILM+rvN+ot98qgEN796jTiQfZfG1KaT0PtFDJ/XFSqti'
