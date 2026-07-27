#!/bin/bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
exec env \
  RS_CI_PROJECT_ROOT="$PROJECT_ROOT" \
  MIN_LINE_COVERAGE="${MIN_LINE_COVERAGE:-93}" \
  MIN_REGION_COVERAGE="${MIN_REGION_COVERAGE:-92}" \
  "$PROJECT_ROOT/.rs-ci/coverage.sh" "$@"
