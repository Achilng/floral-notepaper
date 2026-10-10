#!/usr/bin/env bash

set -euo pipefail

commit=""
tag=""
context=""

usage() {
  echo "Usage: $0 --commit COMMIT --tag TAG --context CONTEXT" >&2
}

while (($# > 0)); do
  case "$1" in
    --commit)
      [[ $# -ge 2 ]] || { usage; exit 2; }
      commit="$2"
      shift 2
      ;;
    --tag)
      [[ $# -ge 2 ]] || { usage; exit 2; }
      tag="$2"
      shift 2
      ;;
    --context)
      [[ $# -ge 2 ]] || { usage; exit 2; }
      context="$2"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 2
      ;;
  esac
done

if [[ -z "$commit" || -z "$tag" || -z "$context" ]]; then
  usage
  exit 2
fi

git fetch --no-tags origin \
  '+refs/heads/main:refs/remotes/origin/main'
if ! git merge-base --is-ancestor "$commit" refs/remotes/origin/main; then
  echo "Release commit $commit is no longer contained in main." >&2
  exit 1
fi

remote_tag_commit="$(
  git ls-remote origin \
    "refs/tags/${tag}" \
    "refs/tags/${tag}^{}" |
    awk '
      $2 ~ /\^\{\}$/ { peeled = $1 }
      $2 !~ /\^\{\}$/ { direct = $1 }
      END {
        if (peeled != "") print peeled
        else print direct
      }
    '
)"
if [[ -z "$remote_tag_commit" || "$remote_tag_commit" != "$commit" ]]; then
  echo "Release tag $tag was deleted or moved before $context." >&2
  echo "Expected $commit, got ${remote_tag_commit:-<missing>}." >&2
  exit 1
fi
