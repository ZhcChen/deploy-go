#!/bin/sh
set -eu

case " $* " in *" StrictHostKeyChecking=yes "*) ;; *) exit 90 ;; esac
case " $* " in *" BatchMode=yes "*) ;; *) exit 91 ;; esac

cat >/dev/null
printf 'fixture stdout\n'
sleep 0.2
printf 'fixture stderr\n' >&2
