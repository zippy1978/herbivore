PING_INTERVAL = 2 * 60  # 2 minutes
CHROME_PING_INTERVAL = 3  # 3 seconds

WEBSOCKET_URLS = [
    "wss://proxy2.wynd.network:4650",
    "wss://proxy2.wynd.network:4444"
]

HEADERS_TO_REPLACE = [
    "origin", "referer", "access-control-request-headers",
    "access-control-request-method", "access-control-allow-origin",
    "cookie", "date", "dnt", "trailer", "upgrade",
]
