"""
Database Connection Management

Features:
- Connection pooling for efficient resource usage
- Automatic retry logic for transient failures
- Health check functionality
"""

import os
import time
import logging
from contextlib import contextmanager
from typing import Optional

import psycopg2
from psycopg2 import pool, OperationalError
from dotenv import load_dotenv

load_dotenv()

logger = logging.getLogger(__name__)

DATABASE_URL = os.getenv("DATABASE_URL", "postgres://user:pass@db:5432/stocks")

# Connection pool configuration
MIN_CONNECTIONS = 1
MAX_CONNECTIONS = 10
MAX_RETRIES = 3
RETRY_DELAY = 1  # seconds


class DatabasePool:
    """Thread-safe connection pool manager.

    The active DSN can be switched at runtime via :meth:`reconfigure` so the
    per-tenant ML batch can point the pool at each ``tenant_<slug>`` database in
    turn. Code that must always talk to the master DB (e.g. the shared RAG
    corpus) should use :func:`get_master_db_connection` instead, which uses a
    separate pool that is never switched.
    """

    _instance: Optional['DatabasePool'] = None
    _pool: Optional[pool.ThreadedConnectionPool] = None
    _dsn: Optional[str] = None

    def __new__(cls):
        if cls._instance is None:
            cls._instance = super().__new__(cls)
        return cls._instance

    def initialize(self):
        """Initialize the connection pool"""
        if self._pool is None:
            try:
                self._pool = pool.ThreadedConnectionPool(
                    MIN_CONNECTIONS,
                    MAX_CONNECTIONS,
                    self._dsn or DATABASE_URL
                )
                logger.info(f"Database pool initialized (min={MIN_CONNECTIONS}, max={MAX_CONNECTIONS})")
            except OperationalError as e:
                logger.error(f"Failed to initialize database pool: {e}")
                raise

    def reconfigure(self, dsn: str):
        """Point the pool at a different database (closes the existing pool)."""
        self.close_all()
        self._dsn = dsn
        self.initialize()

    def get_connection(self):
        """Get a connection from the pool with retry logic"""
        if self._pool is None:
            self.initialize()

        for attempt in range(MAX_RETRIES):
            try:
                conn = self._pool.getconn()
                # Test the connection
                with conn.cursor() as cur:
                    cur.execute("SELECT 1")
                return conn
            except OperationalError as e:
                logger.warning(f"Connection attempt {attempt + 1} failed: {e}")
                if attempt < MAX_RETRIES - 1:
                    time.sleep(RETRY_DELAY)
                else:
                    raise

    def return_connection(self, conn):
        """Return a connection to the pool"""
        if self._pool is not None and conn is not None:
            try:
                self._pool.putconn(conn)
            except Exception as e:
                logger.error(f"Error returning connection to pool: {e}")

    def close_all(self):
        """Close all connections in the pool"""
        if self._pool is not None:
            self._pool.closeall()
            self._pool = None
            logger.info("Database pool closed")

    def health_check(self) -> dict:
        """Check database connectivity"""
        try:
            conn = self.get_connection()
            with conn.cursor() as cur:
                cur.execute("SELECT version()")
                version = cur.fetchone()[0]
            self.return_connection(conn)
            return {
                "status": "healthy",
                "database": "connected",
                "version": version.split(",")[0]
            }
        except Exception as e:
            return {
                "status": "unhealthy",
                "database": "disconnected",
                "error": str(e)
            }


# Global pool instance (DSN switchable per tenant for the ML batch)
_db_pool = DatabasePool()


def reconfigure_pool(dsn: str):
    """Switch the shared pool to a different database (used by per-tenant ML)."""
    _db_pool.reconfigure(dsn)


# ---------------------------------------------------------------------------
# Master-pinned pool — always the master DB, never switched. Used by code that
# must read master-only tables (the shared RAG corpus) even while the main pool
# is temporarily pointed at a tenant DB during an ML run.
# ---------------------------------------------------------------------------
_MASTER_DSN = DATABASE_URL
_master_pool: Optional[pool.ThreadedConnectionPool] = None


def _get_master_pool() -> pool.ThreadedConnectionPool:
    global _master_pool
    if _master_pool is None:
        _master_pool = pool.ThreadedConnectionPool(1, 5, _MASTER_DSN)
        logger.info("Master DB pool initialized")
    return _master_pool


@contextmanager
def get_master_db_connection():
    """Context manager yielding a connection to the master DB (RAG corpus)."""
    p = _get_master_pool()
    conn = p.getconn()
    try:
        yield conn
    finally:
        p.putconn(conn)


def get_connection():
    """Get a database connection from the pool"""
    return _db_pool.get_connection()


def return_connection(conn):
    """Return a connection to the pool"""
    _db_pool.return_connection(conn)


def close_pool():
    """Close all connections"""
    _db_pool.close_all()


def check_database_health() -> dict:
    """Check database health"""
    return _db_pool.health_check()


@contextmanager
def get_db_connection():
    """Context manager for database connections - automatically returns to pool"""
    conn = get_connection()
    try:
        yield conn
    finally:
        return_connection(conn)
