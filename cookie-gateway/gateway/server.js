const { WebSocketServer, WebSocket } = require("ws");

const PORT = 3456;
const TEST_URL = process.env.TEST_URL || "https://example.com";
const TEST_DOMAIN = process.env.TEST_DOMAIN || "example.com";
const OPEN_URL_DELAY_MS = Number(process.env.OPEN_URL_DELAY_MS || 2_000);
const GET_COOKIES_DELAY_MS = Number(process.env.GET_COOKIES_DELAY_MS || 5_000);
const wss = new WebSocketServer({ port: PORT });

console.log(`[Gateway] WebSocket server listening at ws://localhost:${PORT}`);
console.log(
  `[Gateway] Test command config: OPEN_URL(${TEST_URL}) in ${OPEN_URL_DELAY_MS}ms, GET_COOKIES(${TEST_DOMAIN}) in ${GET_COOKIES_DELAY_MS}ms`
);

function printJson(prefix, value) {
  console.log(`${prefix} ${JSON.stringify(value, null, 2)}`);
}

function sendCommand(ws, action, payload) {
  const requestId = `${action.toLowerCase()}-${Date.now()}-${Math.random()
    .toString(16)
    .slice(2, 8)}`;

  const message = {
    requestId,
    action,
    ...payload,
  };

  ws.send(JSON.stringify(message));
  printJson(`[Gateway] -> Sent ${action}`, message);
}

wss.on("connection", (ws, req) => {
  console.log(`[Gateway] Client connected from ${req.socket.remoteAddress}`);

  ws.on("message", (raw) => {
    let message;
    try {
      message = JSON.parse(raw.toString());
    } catch (error) {
      console.log(`[Gateway] <- Non-JSON message: ${raw.toString()}`);
      return;
    }

    if (message.type === "PING") {
      console.log(`[Gateway] <- PING ${message.timestamp || ""}`.trim());
      ws.send(
        JSON.stringify({
          type: "PONG",
          timestamp: new Date().toISOString(),
        })
      );
      return;
    }

    if (message.type === "ACTION_RESULT") {
      if (message.action === "GET_COOKIES") {
        const cookies = Array.isArray(message?.result?.cookies)
          ? message.result.cookies
          : [];
        const count = Number.isInteger(message?.result?.count)
          ? message.result.count
          : cookies.length;
        const cookieNames = cookies.map((cookie) => `${cookie.name}@${cookie.domain}`);
        console.log(`[Gateway] <- GET_COOKIES count=${count}`);
        if (cookieNames.length > 0) {
          console.log(`[Gateway] <- GET_COOKIES names: ${cookieNames.join(", ")}`);
        }
      }
      printJson(`[Gateway] <- ACTION_RESULT`, message);
      return;
    }

    printJson(`[Gateway] <- Message`, message);
  });

  ws.on("close", () => {
    console.log("[Gateway] Client disconnected");
  });

  ws.on("error", (error) => {
    console.error("[Gateway] Client socket error:", error.message);
  });

  setTimeout(() => {
    if (ws.readyState === WebSocket.OPEN) {
      sendCommand(ws, "OPEN_URL", { url: TEST_URL });
    }
  }, OPEN_URL_DELAY_MS);

  setTimeout(() => {
    if (ws.readyState === WebSocket.OPEN) {
      sendCommand(ws, "GET_COOKIES", { domain: TEST_DOMAIN });
    }
  }, GET_COOKIES_DELAY_MS);
});

wss.on("error", (error) => {
  console.error("[Gateway] Server error:", error.message);
});
