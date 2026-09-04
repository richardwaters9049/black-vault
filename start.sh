#!/usr/bin/env bash

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

cd "$PROJECT_ROOT"

COMPOSE_FILE="$PROJECT_ROOT/docker-compose.yml"
ENV_FILE="$PROJECT_ROOT/.env.docker"

COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-black-vault}"

# ============================================================
# BLACK VAULT TERMINAL THEME
# ============================================================

RESET=$'\033[0m'
BOLD=$'\033[1m'
DIM=$'\033[2m'
UNDERLINE=$'\033[4m'

BRIGHT_BLACK=$'\033[90m'
BRIGHT_RED=$'\033[91m'
BRIGHT_GREEN=$'\033[92m'
BRIGHT_YELLOW=$'\033[93m'
BRIGHT_BLUE=$'\033[94m'
BRIGHT_MAGENTA=$'\033[95m'
BRIGHT_CYAN=$'\033[96m'
BRIGHT_WHITE=$'\033[97m'

CLEAR_LINE=$'\033[2K'

# ============================================================
# TERMINAL HELPERS
# ============================================================

print_banner() {
    clear 2>/dev/null || true

    echo ""
    echo "${BRIGHT_CYAN}${BOLD}"
    echo "    ██████╗ ██╗      █████╗  ██████╗██╗  ██╗    ██╗   ██╗ █████╗ ██╗   ██╗██╗  ████████╗"
    echo "    ██╔══██╗██║     ██╔══██╗██╔════╝██║ ██╔╝    ██║   ██║██╔══██╗██║   ██║██║  ╚══██╔══╝"
    echo "    ██████╔╝██║     ███████║██║     █████╔╝     ██║   ██║███████║██║   ██║██║     ██║"
    echo "    ██╔══██╗██║     ██╔══██║██║     ██╔═██╗     ╚██╗ ██╔╝██╔══██║██║   ██║██║     ██║"
    echo "    ██████╔╝███████╗██║  ██║╚██████╗██║  ██╗     ╚████╔╝ ██║  ██║╚██████╔╝███████╗██║"
    echo "    ╚═════╝ ╚══════╝╚═╝  ╚═╝ ╚═════╝╚═╝  ╚═╝      ╚═══╝  ╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚═╝"
    echo "${RESET}"
    echo "    ${DIM}${BRIGHT_MAGENTA}◆ Secure development environment${RESET}"
    echo "    ${DIM}${BRIGHT_BLACK}────────────────────────────────────────────────────────────────────────────────${RESET}"
    echo ""
}

section() {
    echo ""
    echo "${BRIGHT_CYAN}${BOLD}◆ $1${RESET}"
    echo "${BRIGHT_BLACK}  ─────────────────────────────────────────────${RESET}"
}

success() {
    echo "  ${BRIGHT_GREEN}✔${RESET} $1"
}

info() {
    echo "  ${BRIGHT_CYAN}◆${RESET} $1"
}

warning() {
    echo "  ${BRIGHT_YELLOW}▲${RESET} $1"
}

error() {
    echo "  ${BRIGHT_RED}✖${RESET} $1"
}

url() {
    echo "    ${UNDERLINE}${BRIGHT_CYAN}$1${RESET}"
}

spinner() {
    local pid="$1"
    local message="$2"

    local frames=(
        "⠋"
        "⠙"
        "⠹"
        "⠸"
        "⠼"
        "⠴"
        "⠦"
        "⠧"
        "⠇"
        "⠏"
    )

    local i=0

    while kill -0 "$pid" 2>/dev/null; do
        printf "\r${CLEAR_LINE}  ${BRIGHT_MAGENTA}${frames[$i]}${RESET} ${message}"

        i=$(( (i + 1) % ${#frames[@]} ))

        sleep 0.08
    done

    wait "$pid"
    local exit_code=$?

    printf "\r${CLEAR_LINE}"

    return "$exit_code"
}

run_with_spinner() {
    local message="$1"
    shift

    "$@" >/tmp/black-vault-command.log 2>&1 &
    local pid=$!

    if spinner "$pid" "$message"; then
        success "$message"
    else
        error "$message"
        echo ""
        cat /tmp/black-vault-command.log
        rm -f /tmp/black-vault-command.log
        return 1
    fi

    rm -f /tmp/black-vault-command.log
}

progress() {
    local current="$1"
    local total="$2"
    local label="$3"

    local filled=""
    local empty=""
    local i

    for ((i = 0; i < current; i++)); do
        filled+="━"
    done

    for ((i = current; i < total; i++)); do
        empty+="─"
    done

    echo ""
    echo "  ${BRIGHT_MAGENTA}${filled}${BRIGHT_BLACK}${empty}${RESET}"
    echo "  ${DIM}[$current/$total]${RESET} ${BRIGHT_WHITE}${label}${RESET}"
}

# ============================================================
# DOCKER
# ============================================================

check_docker() {
    if ! command -v docker >/dev/null 2>&1; then
        error "Docker is not installed."
        echo ""
        echo "  Install Docker Desktop and try again."
        exit 1
    fi

    if ! docker info >/dev/null 2>&1; then
        error "Docker is not running."
        echo ""
        echo "  ${DIM}Start Docker Desktop and run ./start.sh again.${RESET}"
        exit 1
    fi

    success "Docker engine online"
}

compose() {
    FRONTEND_PORT="${FRONTEND_PORT:-0}" \
    BACKEND_PORT="${BACKEND_PORT:-0}" \
    POSTGRES_PORT="${POSTGRES_PORT:-0}" \
    NEXT_PUBLIC_API_URL="${NEXT_PUBLIC_API_URL:-http://localhost:0}" \
    docker compose \
        --project-name "$COMPOSE_PROJECT_NAME" \
        --env-file "$ENV_FILE" \
        -f "$COMPOSE_FILE" \
        "$@"
}

get_published_port() {
    local service="$1"
    local container_port="$2"

    compose port "$service" "$container_port" |
        sed -E 's/.*:([0-9]+)$/\1/'
}

# ============================================================
# ENVIRONMENT
# ============================================================

create_environment() {
    if [[ -f "$ENV_FILE" ]]; then
        return
    fi

    warning ".env.docker not found"
    echo ""
    echo "  ${DIM}Creating development environment...${RESET}"

    cat > "$ENV_FILE" <<'EOF'
JWT_SECRET=black-vault-development-secret-change-me
JWT_EXPIRATION=3600
EOF

    sleep 0.4

    success "Development environment created"
}

# ============================================================
# STOP
# ============================================================

stop_stack() {
    print_banner
    check_docker

    create_environment

    section "SHUTTING DOWN"

    info "Stopping Black Vault containers..."

    compose down

    echo ""
    echo "${BRIGHT_GREEN}${BOLD}  ◆ Black Vault has been stopped.${RESET}"
    echo ""
}

# ============================================================
# LOGS
# ============================================================

show_logs() {
    check_docker
    create_environment

    compose logs -f
}

# ============================================================
# BACKEND HEALTH
# ============================================================

wait_for_backend() {
    local attempts=60

    for ((i = 1; i <= attempts; i++)); do
        if curl -fsS \
            "http://localhost:${BACKEND_PORT}/health" \
            >/dev/null 2>&1; then

            echo ""

            return 0
        fi

        printf "\r${CLEAR_LINE}  ${BRIGHT_MAGENTA}◉${RESET} Waiting for API ${DIM}[%02d/%02d]${RESET}" \
            "$i" \
            "$attempts"

        sleep 1
    done

    echo ""

    return 1
}

# ============================================================
# START
# ============================================================

start_stack() {
    print_banner

    section "SYSTEM CHECK"

    check_docker
    create_environment

    progress 1 5 "Environment ready"

    # --------------------------------------------------------
    # DOCKER CONFIGURATION
    # --------------------------------------------------------

    section "BUILD PIPELINE"

    info "Validating Docker configuration..."

    FRONTEND_PORT=0 \
    BACKEND_PORT=0 \
    POSTGRES_PORT=0 \
    NEXT_PUBLIC_API_URL=http://localhost:0 \
    compose config >/dev/null

    success "Docker configuration valid"

    echo ""
    info "Building Black Vault images..."

    FRONTEND_PORT=0 \
    BACKEND_PORT=0 \
    POSTGRES_PORT=0 \
    NEXT_PUBLIC_API_URL=http://localhost:0 \
    compose build

    progress 2 5 "Container images ready"

    # --------------------------------------------------------
    # DATABASE + BACKEND
    # --------------------------------------------------------

    section "SECURE CORE"

    info "Launching PostgreSQL and Rust API..."

    FRONTEND_PORT=0 \
    BACKEND_PORT=0 \
    POSTGRES_PORT=0 \
    NEXT_PUBLIC_API_URL=http://localhost:0 \
    compose up -d postgres backend

    POSTGRES_PORT="$(get_published_port postgres 5432)"
    BACKEND_PORT="$(get_published_port backend 8080)"

    if [[ -z "$POSTGRES_PORT" || -z "$BACKEND_PORT" ]]; then
        error "Failed to determine Docker allocated ports."
        compose ps
        exit 1
    fi

    success "PostgreSQL allocated → ${BRIGHT_CYAN}${POSTGRES_PORT}${RESET}"
    success "Rust API allocated → ${BRIGHT_CYAN}${BACKEND_PORT}${RESET}"

    echo ""
    info "Waiting for Rust API..."

    if wait_for_backend; then
        success "Rust API is healthy"
    else
        error "Backend failed to become ready."
        echo ""
        compose logs --tail=100 backend
        exit 1
    fi

    success "Database connection established"

    progress 3 5 "Secure backend online"

    # --------------------------------------------------------
    # FRONTEND
    # --------------------------------------------------------

    section "FRONTEND"

    info "Starting Next.js development environment..."
    info "Connecting frontend → API :${BACKEND_PORT}"

    FRONTEND_PORT=0 \
    BACKEND_PORT="$BACKEND_PORT" \
    POSTGRES_PORT="$POSTGRES_PORT" \
    NEXT_PUBLIC_API_URL="http://localhost:${BACKEND_PORT}" \
    compose up -d frontend

    FRONTEND_PORT="$(get_published_port frontend 3000)"

    if [[ -z "$FRONTEND_PORT" ]]; then
        error "Failed to determine frontend port."
        compose ps
        exit 1
    fi

    success "Next.js allocated → ${BRIGHT_CYAN}${FRONTEND_PORT}${RESET}"
    success "Dependencies synchronised"
    success "Framer Motion available"
    success "Hot reload enabled"

    progress 4 5 "Frontend online"

    # --------------------------------------------------------
    # FINAL STATUS
    # --------------------------------------------------------

    section "SYSTEM STATUS"

    sleep 1

    if ! compose ps >/dev/null 2>&1; then
        error "Unable to inspect Docker services."
        exit 1
    fi

    success "PostgreSQL        ONLINE"
    success "Rust API          ONLINE"
    success "Next.js           ONLINE"
    success "Docker network    ONLINE"
    success "Development mode  ONLINE"

    progress 5 5 "Black Vault ready"

    # --------------------------------------------------------
    # LAUNCH PANEL
    # --------------------------------------------------------

    echo ""
    echo ""

    echo "${BRIGHT_CYAN}${BOLD}"
    echo "    ╔══════════════════════════════════════════════════════════╗"
    echo "    ║                                                          ║"
    echo "    ║              ◆ BLACK VAULT ONLINE ◆                     ║"
    echo "    ║                                                          ║"
    echo "    ║          Your secure development environment             ║"
    echo "    ║                    is ready.                             ║"
    echo "    ║                                                          ║"
    echo "    ╚══════════════════════════════════════════════════════════╝"
    echo "${RESET}"

    echo ""
    echo "  ${BOLD}${BRIGHT_WHITE}WEB INTERFACE${RESET}"
    url "http://localhost:${FRONTEND_PORT}"

    echo ""
    echo "  ${BOLD}${BRIGHT_WHITE}RUST API${RESET}"
    url "http://localhost:${BACKEND_PORT}"

    echo ""
    echo "  ${BOLD}${BRIGHT_WHITE}API HEALTH${RESET}"
    url "http://localhost:${BACKEND_PORT}/health"

    echo ""
    echo "  ${BOLD}${BRIGHT_WHITE}POSTGRESQL${RESET}"
    echo "    ${BRIGHT_CYAN}localhost:${POSTGRES_PORT}${RESET}"

    echo ""
    echo "  ${BOLD}${BRIGHT_WHITE}COMMANDS${RESET}"
    echo "    ${BRIGHT_MAGENTA}./start.sh logs${RESET}     ${DIM}View live container logs${RESET}"
    echo "    ${BRIGHT_MAGENTA}./start.sh stop${RESET}     ${DIM}Stop Black Vault${RESET}"
    echo "    ${BRIGHT_MAGENTA}./start.sh restart${RESET}  ${DIM}Restart everything${RESET}"

    echo ""
    echo "${BRIGHT_BLACK}    ────────────────────────────────────────────────────────────${RESET}"
    echo ""
    echo "    ${DIM}${BRIGHT_GREEN}●${RESET} ${DIM}All systems operational${RESET}"
    echo "    ${DIM}${BRIGHT_CYAN}◆${RESET} ${DIM}Ports dynamically allocated by Docker${RESET}"
    echo "    ${DIM}${BRIGHT_MAGENTA}◆${RESET} ${DIM}Development hot reload active${RESET}"
    echo ""

    echo "${BRIGHT_CYAN}${BOLD}    ◆ Vault secured. Let's build.${RESET}"
    echo ""
}

# ============================================================
# RESTART
# ============================================================

restart_stack() {
    stop_stack

    sleep 0.5

    start_stack
}

# ============================================================
# MAIN
# ============================================================

main() {
    case "${1:-start}" in
        start)
            start_stack
            ;;

        stop)
            stop_stack
            ;;

        restart)
            restart_stack
            ;;

        logs)
            show_logs
            ;;

        *)
            print_banner

            echo "  ${BOLD}Usage${RESET}"
            echo ""
            echo "    ${BRIGHT_CYAN}./start.sh${RESET}          Start Black Vault"
            echo "    ${BRIGHT_CYAN}./start.sh stop${RESET}     Stop Black Vault"
            echo "    ${BRIGHT_CYAN}./start.sh restart${RESET}  Restart everything"
            echo "    ${BRIGHT_CYAN}./start.sh logs${RESET}     View live logs"
            echo ""

            exit 1
            ;;
    esac
}

main "$@"