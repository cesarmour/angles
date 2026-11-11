#!/bin/sh
# Roda o jogo em modo de roteiro: ./run.sh <shots|run> [velocidade]
DIR="$(cd "$(dirname "$0")" && pwd)"
OUT="${CQB_OUT:-$DIR/shots}"
mkdir -p "$OUT"
caffeinate -u -t 1
BEVY_ASSET_ROOT="$DIR" CQB_SCRIPT="$1" CQB_SPEED="${2:-1}" CQB_OUT="$OUT" caffeinate -d perl -e 'alarm shift; exec @ARGV' "${CQB_ALARM:-240}" "$DIR/target/${PROFILE:-debug}/ponto-de-entrada" 2>&1 | sed 's/\x1b\[[0-9;]*m//g' | grep -E "BOT|ERROR|WARN|panic" | cut -c1-500
