#!/bin/sh
set -eu

args=" $* "
case "$args" in
  *" StrictHostKeyChecking=yes "*) ;;
  *) echo "missing strict host key checking" >&2; exit 2 ;;
esac
case "$args" in
  *" UserKnownHostsFile="*) ;;
  *) echo "missing known hosts file" >&2; exit 2 ;;
esac
case "$args" in
  *" -i "*) ;;
  *) echo "missing identity file" >&2; exit 2 ;;
esac

cat >/dev/null
printf '%s\n' 'os_name=Linux' 'architecture=x86_64' 'disk_available_bytes=1048576'
