#!/usr/bin/env bash

# Drop the database
docker compose -f sql-compose.yaml down --volumes
docker compose -f sql-compose.yaml up -d

# Wait until PostgreSQL is ready
echo "Waiting for PostgreSQL to start..."
until docker exec my_postgres pg_isready -U username; do
  sleep 1
done

(cd ./packages/adapters/storage && sqlx migrate run)
