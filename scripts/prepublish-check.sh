#!/usr/bin/env bash
# Checks to run before making the repository public.
# Everything in the git history becomes visible, not just the current files.
# Usage: scripts/prepublish-check.sh   (from the repository root)
set -u
cd "$(git rev-parse --show-toplevel)" || exit 1
fail=0

echo "== 1. Commit author emails (public in every commit) =="
git log --all --format='%an <%ae>' | sort -u
if git log --all --format='%ae' | grep -qv 'users.noreply.github.com'; then
  echo "   NOTE: some commits use a non-noreply email; it will be public. OK if you don't mind."
fi

echo
echo "== 2. Anything that looks like a secret, anywhere in history =="
hits=$(git log -p --all -- . ':(exclude)scripts/prepublish-check.sh' | grep -inE 'api[_-]?key|secret|password|passwd|token|BEGIN (RSA|OPENSSH|EC) PRIVATE' \
        | grep -viE 'GITHUB_TOKEN|GH_TOKEN|secrets\.GITHUB|walkTokens|token\.(type|href)' | head -20)
if [ -n "$hits" ]; then echo "$hits"; echo "   CHECK the lines above."; fail=1; else echo "   none found"; fi

echo
echo "== 3. Your git email inside any committed file =="
me=$(git config user.email)
if [ -n "$me" ] && git grep -n -F "$me" $(git rev-list --all) -- . >/dev/null 2>&1; then
  git grep -n -F "$me" $(git rev-list --all) -- . | head -5; echo "   CHECK the lines above."; fail=1
else
  echo "   none found"
fi

echo
echo "== 4. Licence =="
if [ -f LICENSE ] && head -1 LICENSE | grep -q 'MIT License'; then
  echo "   LICENSE present: $(head -1 LICENSE), $(grep -m1 Copyright LICENSE)"
else
  echo "   no MIT LICENSE file"; fail=1
fi
grep -q '"license": *"MIT"' package.json && echo "   package.json: MIT" || { echo "   package.json licence is not MIT"; fail=1; }
grep -q '^license *= *"MIT"' src-tauri/Cargo.toml && echo "   Cargo.toml: MIT" || { echo "   Cargo.toml has no license = \"MIT\""; fail=1; }

echo
echo "== 5. Large or unexpected files in history (>5 MB) =="
big=$(git rev-list --objects --all | git cat-file --batch-check='%(objecttype) %(objectname) %(objectsize) %(rest)' \
      | awk '$1=="blob" && $3>5*1024*1024 {printf "   %.1f MB  %s\n", $3/1048576, $4}')
[ -n "$big" ] && echo "$big" || echo "   none"

echo
echo "== 6. Working tree clean and pushed =="
git status -sb | head -5

echo
if [ $fail -eq 0 ]; then echo "RESULT: OK to make public."; else echo "RESULT: review the items marked CHECK / missing above."; fi
exit $fail
