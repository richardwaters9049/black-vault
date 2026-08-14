#!/usr/bin/env bash

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$PROJECT_ROOT"

echo ""
echo "=========================================="
echo "          BLACK VAULT DOCKER"
echo "=========================================="
echo ""

if ! command -v docker >/dev/null 2>&1; then
    echo "Error: Docker is not installed."
    exit 1
fi

if ! docker info >/dev/null 2>&1; then
    echo "Error: Docker is not running."
    echo "Please start Docker Desktop and try again."
    exit 1
fi

if [[ ! -f "$PROJECT_ROOT/.env.docker" ]]; then
    echo "Error: .env.docker does not exist."
    echo ""
    echo "Create it with:"
    echo ""
    echo "JWT_SECRET=your-development-secret"
    echo "JWT_EXPIRATION=3600"
    echo ""
    exit 1
fi

echo "[1/3] Validating Docker configuration..."

docker compose \
    --env-file "$PROJECT_ROOT/.env.docker" \
    -f "$PROJECT_ROOT/docker-compose.yml" \
    config >/dev/null

echo "      Configuration valid."
echo ""

echo "[2/3] Building containers..."

docker compose \
    --env-file "$PROJECT_ROOT/.env.docker" \
    -f "$PROJECT_ROOT/docker-compose.yml" \
    build

echo ""

echo "[3/3] Starting Black Vault..."

docker compose \
    --env-file "$PROJECT_ROOT/.env.docker" \
    -f "$PROJECT_ROOT/docker-compose.yml" \
    up -d

echo ""
echo "=========================================="
echo "       BLACK VAULT IS RUNNING"
echo "=========================================="
echo ""
echo "Frontend:"
echo "  http://localhost:3000"
echo ""
echo "Backend:"
echo "  http://localhost:8080"
echo ""
echo "Health:"
echo "  http://localhost:8080/health"
echo ""
echo "PostgreSQL:"
echo "  localhost:5432"
echo ""
echo "View logs:"
echo "  docker compose --env-file .env.docker logs -f"
echo ""
echo "Stop:"
echo "  docker compose --env-file .env.docker down"
echo ""
echo "=========================================="
echo ""