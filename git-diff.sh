#!/bin/bash

if [ "$#" != 2 ]; then
    echo "Usage: $(basename $0) [branch] [file]"
    exit 1
fi

branch="$1"
file="$2"

git diff --unified=16 --no-index \
  <(git show "$branch:$file" | jq -S 'sort_by(.course_id)') \
  <(jq -S 'sort_by(.course_id)' "$file")
