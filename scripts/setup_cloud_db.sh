#!/bin/bash
set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Setting up PostgreSQL for Cloud/Container Environment...${NC}"

# 1. Check/Install PostgreSQL
if ! command -v psql &> /dev/null; then
    echo "PostgreSQL not found. Attempting installation..."
    
    # Check package manager
    if command -v apt-get &> /dev/null; then
        sudo apt-get update
        sudo apt-get install -y postgresql postgresql-contrib
    elif command -v apk &> /dev/null; then
        sudo apk add postgresql postgresql-contrib
    else
        echo "Error: Unsupported package manager. Please install PostgreSQL manually."
        exit 1
    fi
else
    echo -e "${GREEN}PostgreSQL is already installed.${NC}"
fi

# 2. Start PostgreSQL Service
echo "Ensuring PostgreSQL service is running..."
if command -v service &> /dev/null; then
    sudo service postgresql start || echo "Service start command failed, trying pg_ctl..."
fi

# Fallback: Try to initialize and start if service command failed or didn't exist (common in minimal containers)
if ! pgrep -x "postgres" > /dev/null; then
    echo "Postgres process not found. Checking for manual initialization..."
    PGdata="/var/lib/postgresql/data"
    
    # If we are root and need to run as postgres
    if [ "$(id -u)" -eq 0 ]; then
        # Ensure directory exists and has correct permissions
        mkdir -p "$PGdata"
        chown postgres:postgres "$PGdata"
        
        # Init DB if empty
        if [ -z "$(ls -A $PGdata)" ]; then
            su - postgres -c "initdb -D $PGdata"
        fi
        
        # Start manually
        su - postgres -c "pg_ctl -D $PGdata -l /var/log/postgresql/logfile start"
    fi
fi

# 3. Wait for readiness
echo "Waiting for PostgreSQL to accept connections..."
RETRIES=0
until sudo -u postgres psql -c '\l' &> /dev/null || [ $RETRIES -eq 10 ]; do
    echo -n "."
    sleep 1
    RETRIES=$((RETRIES+1))
done
echo ""

if [ $RETRIES -eq 10 ]; then
    echo "Error: PostgreSQL did not start in time."
    exit 1
fi

# 4. Configure User and Database
echo "Configuring database..."

# Set password for default user 'postgres' to 'postgres'
sudo -u postgres psql -c "ALTER USER postgres WITH PASSWORD 'postgres';"

# Create DB if not exists
if ! sudo -u postgres psql -lqt | cut -d \| -f 1 | grep -qw apex_stack; then
    echo "Creating database 'apex_stack'..."
    sudo -u postgres createdb apex_stack
else
    echo -e "${GREEN}Database 'apex_stack' already exists.${NC}"
fi

# 5. Output Success
echo -e "${GREEN}Setup Complete!${NC}"
echo ""
echo "Your database is running on default port 5432."
echo "Please update your .env file to use port 5432 (Docker uses 5433):"
echo ""
echo "DATABASE_URL=postgres://postgres:postgres@localhost:5432/apex_stack"
echo ""
