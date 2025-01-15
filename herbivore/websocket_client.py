import logging
import time
import json
import base64
import asyncio
import uuid
import aiohttp
import websockets
from .constants import *
from .utils import ResponseProcessor
from fake_useragent import UserAgent

async def perform_http_request(params: dict, websocket, logger: logging.Logger) -> dict:
    logger.info(f"Performing {params['method']} request to {params['url']}")
    headers = params.get('headers', {})
    url = params['url']
    method = params['method']
    body = params.get('body')

    headers = {
        k: v for k, v in headers.items() 
        if k.lower() not in HEADERS_TO_REPLACE
    }

    async with aiohttp.ClientSession() as session:
        request_kwargs = {
            'method': method,
            'headers': headers,
            'allow_redirects': False
        }
        
        if body:
            body_data = base64.b64decode(body)
            request_kwargs['data'] = body_data

        try:
            async with session.request(url=url, **request_kwargs) as response:
                response_headers = dict(response.headers)
                response_body = await response.read()
                
                logger.info(f"Response size: {len(response_body)} bytes")
                
                return {
                    'url': str(response.url),
                    'status': response.status,
                    'status_text': response.reason,
                    'headers': response_headers,
                    'body': base64.b64encode(response_body).decode('utf-8')
                }
        except Exception as e:
            logger.error(f"Error in HTTP request: {str(e)}")
            return None

class WebSocketClient:
    def __init__(self, user_id: str,node_type: str, logger: logging.Logger):
        self.websocket = None
        self.last_live_connection_timestamp = time.time()
        self.retries = 0
        self.response_processor = ResponseProcessor()
        self.user_id = user_id
        self.node_type = node_type
        self.logger = logger
    async def connect(self):
        websocket_url = WEBSOCKET_URLS[self.retries % len(WEBSOCKET_URLS)]
        self.websocket = await websockets.connect(websocket_url)
        self.logger.info(f"Connected to {websocket_url}")
        self.logger.info(f"Node type: {self.node_type}")
        
    async def authenticate(self):
        try:
            self.logger.info("Authenticating...")
            auth_message = {
                "id": str(uuid.uuid4()),
                "origin_action": "AUTH",
                "result": {
                    "browser_id": str(uuid.uuid4()),
                    "user_id": self.user_id,
                    "user_agent": UserAgent().random,
                    "timestamp": int(time.time()),
                    "device_type": "extension",
                    "version": "4.26.2",
                    "extension_id": "ilehaonighjijnmpnagapkhpcdbhclfg"
                }
            }
            
            if self.node_type == '1.25x':
                auth_message['result'].update({
                    "extension_id": "lkbnfiajjmbhnfledhphioinpickokdi",
                })
            elif self.node_type == '2x':
                auth_message['result'].update({
                "device_type": "desktop",
                "version": "4.30.0",
                })
                auth_message['result'].pop("extension_id")
                
            await self.websocket.send(json.dumps(auth_message))
        except Exception as e:
            self.logger.error(f"Authentication error: {str(e)}")

    async def handle_message(self, message):
        self.logger.info(f"Received message: {message}")
        try:
            parsed_message = json.loads(message)
            action = parsed_message.get('action')
            
            if action == 'HTTP_REQUEST':
                result = await perform_http_request(parsed_message['data'], self.websocket, self.logger)
                await self.send_response(parsed_message['id'], action, result)
            elif action == 'PING':
                await self.send_response(parsed_message['id'], action, {})

                
        except Exception as e:
            self.logger.error(f"Error handling message: {str(e)}")

    async def send_response(self, msg_id: str, origin_action: str, result: dict):
        response = {
            'id': msg_id,
            'origin_action': origin_action,
            'result': result
        }
        await self.websocket.send(json.dumps(response))

    async def start(self):
        while True:
            try:
                await self.connect()
                await self.authenticate()
                self.logger.info("Waiting for messages...")
                async for message in self.websocket:
                    self.last_live_connection_timestamp = time.time()
                    await self.handle_message(message)
            except Exception as e:
                self.logger.error(f"WebSocket error: {str(e)}")
                self.retries += 1
                await asyncio.sleep(1)