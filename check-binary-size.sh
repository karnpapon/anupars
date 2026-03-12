#!/bin/bash

# limit to 900 KB
LIMIT=$((900 * 1024)) 

cargo build --release && \
  awk -v limit="$LIMIT" '{if($5>limit) {print "TOO BIG: "$5" bytes"; exit 1} else print "OK: "$5" bytes"}' \
  <(ls -l target/release/anupars)
