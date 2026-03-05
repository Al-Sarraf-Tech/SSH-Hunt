#!/usr/bin/env bash
set -euo pipefail

runner_compose='docker-compose.runner.yml'

all_runs_on="$(grep -RIn --include='*.yml' 'runs-on:' .github/workflows || true)"
if [[ -z "${all_runs_on}" ]]; then
  echo "No runs-on directives found in workflows."
  exit 1
fi

violations="$(printf '%s\n' "${all_runs_on}" | grep -Ev 'vars\.SSH_HUNT_RUNNER_LABELS' || true)"
if [[ -n "${violations}" ]]; then
  echo "Runner directive violation detected. Every workflow job must use the runner selector variable:"
  echo "  runs-on: \${{ fromJSON(vars.SSH_HUNT_RUNNER_LABELS ... ) }}"
  echo ""
  echo "Violations:"
  printf '%s\n' "${violations}"
  exit 1
fi

missing_fallback="$(printf '%s\n' "${all_runs_on}" | grep -Ev '\["ubuntu-latest"\]' || true)"
if [[ -n "${missing_fallback}" ]]; then
  echo "Runner selector must keep a GitHub-hosted fallback for forks/clones."
  echo "Missing fallback in:"
  printf '%s\n' "${missing_fallback}"
  exit 1
fi

if [[ ! -f "${runner_compose}" ]]; then
  echo "Missing ${runner_compose}; self-hosted runner directive cannot be enforced."
  exit 1
fi

ephemeral_service_count="$(grep -En '^[[:space:]]{2}github-runner-ephemeral-[0-9]+:' "${runner_compose}" | wc -l | tr -d ' ')"
if [[ "${ephemeral_service_count}" != "4" ]]; then
  echo "Runner directive violation: ${runner_compose} must define exactly 4 ephemeral runners."
  echo "Found ${ephemeral_service_count} ephemeral services."
  exit 1
fi

for idx in 1 2 3 4; do
  if ! grep -En "^[[:space:]]{2}github-runner-ephemeral-${idx}:" "${runner_compose}" >/dev/null; then
    echo "Missing github-runner-ephemeral-${idx} in ${runner_compose}."
    exit 1
  fi
done

host_network_count="$(grep -En '^[[:space:]]{4}network_mode:[[:space:]]host$' "${runner_compose}" | wc -l | tr -d ' ')"
if [[ "${host_network_count}" != "5" ]]; then
  echo "Runner directive violation: all 5 runner services must set network_mode: host."
  echo "Found ${host_network_count} host-network declarations."
  exit 1
fi

if ! grep -En '^runner-up: runner-env' Makefile >/dev/null; then
  echo "Runner directive violation: Makefile runner-up target is missing."
  exit 1
fi

if ! grep -En 'docker compose -f docker-compose\.runner\.yml up -d' Makefile >/dev/null; then
  echo "Runner directive violation: runner-up must bring up docker-compose.runner.yml services."
  exit 1
fi

echo "Runner selector and ephemeral pool directive verified across workflows."
