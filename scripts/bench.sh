#!/usr/bin/env sh
# bench - Bad benchmarking

PAR=100
hyperfine "seq 1 $PAR | xargs -n1 -P $PAR vrsctl -c '(+ 1 (+ 1 (+ 1 1)))'"
