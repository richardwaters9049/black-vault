#!/usr/bin/env bash

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DESKTOP_ROOT="$PROJECT_ROOT/desktop"
DESKTOP_APP="$DESKTOP_ROOT/src-tauri/target/release/bundle/macos/Black Vault.app"

error() {
    printf 'Black Vault: %s\n' "$*" >&2
}

success() {
    printf 'Black Vault: %s\n' "$*"
}

desktop_npm() {
    if ! command -v npm >/dev/null 2>&1; then
        error "Node.js/npm is required for desktop development."
        return 1
    fi
    (cd "$DESKTOP_ROOT" && npm run "$1")
}

launch_desktop() {
    if [[ "$(uname -s)" != "Darwin" ]]; then
        error "The current local release launcher targets macOS only."
        return 1
    fi
    if [[ ! -d "$DESKTOP_APP" ]]; then
        error "Native application not built. Run ./start.sh build first."
        return 1
    fi
    open "$DESKTOP_APP"
    success "Black Vault launched"
}

main() {
    case "${1:-start}" in
        start|black-v-start)
            if [[ -s "$DESKTOP_ROOT/.env.development.local" ]]; then
                success "Local development configuration detected; starting development app."
                desktop_npm desktop:dev
            else
                launch_desktop
            fi
            ;;
        release) launch_desktop ;;
        dev) desktop_npm desktop:dev ;;
        build) desktop_npm desktop:build ;;
        test)
            desktop_npm typecheck
            desktop_npm test
            desktop_npm test:rust
            ;;
        help|-h|--help)
            echo "Black Vault — native desktop"
            echo "  ./start.sh          Launch the active desktop application"
            echo "  ./start.sh dev      Run the Tauri development application"
            echo "  ./start.sh release  Launch the compiled macOS application"
            echo "  ./start.sh build    Build the native desktop bundle"
            echo "  ./start.sh test     Run desktop TypeScript, UI and Rust checks"
            echo
            echo "Historical web stack: ./legacy/start.sh help"
            ;;
        *) error "Unknown command: $1"; echo "Run ./start.sh help"; return 2 ;;
    esac
}

main "$@"
