import asyncio
import json
from typing import Dict

class Mutex:
    def __init__(self):
        self._lock = asyncio.Lock()
    
    async def __aenter__(self):
        await self._lock.acquire()
        return self
    
    async def __aexit__(self, exc_type, exc_val, exc_tb):
        self._lock.release()

class LogsTransporter:
    @staticmethod
    async def send_logs(websocket, logs):
        await websocket.send(json.dumps({
            "action": "LOGS",
            "data": logs
        }))

class ResponseProcessor:
    def __init__(self):
        self._cookie_mutex = Mutex()
        self._redirect_mutex = Mutex()
        self._cookie_storage: Dict[str, str] = {}
        self._redirect_data_storage: Dict[str, str] = {}

    async def get_response_cookies(self, request_id: str, timeout_ms: int) -> str:
        async with self._cookie_mutex:
            if request_id in self._cookie_storage:
                return self._cookie_storage[request_id]
            
            try:
                async with asyncio.timeout(timeout_ms/1000):
                    while request_id not in self._cookie_storage:
                        await asyncio.sleep(0.1)
                    return self._cookie_storage[request_id]
            except asyncio.TimeoutError:
                return ""

    async def set_response_cookies(self, request_id: str, cookies: str):
        async with self._cookie_mutex:
            self._cookie_storage[request_id] = cookies 