import asyncpg
import asyncio

__all__ = ["connect"]

async def _connect_async(dsn: str):
    return await asyncpg.connect(dsn)

def connect(dsn: str):
    """Connect synchronously to SerinDB using asyncpg under the hood."""
    loop = asyncio.new_event_loop()
    conn = loop.run_until_complete(_connect_async(dsn))
    return conn 