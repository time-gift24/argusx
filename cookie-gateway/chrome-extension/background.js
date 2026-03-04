const GATEWAY_WS_URL = "ws://localhost:3456";
const HEARTBEAT_INTERVAL_MS = 20_000;
const RECONNECT_DELAY_MS = 5_000;
const CONNECTION_CHECK_ALARM = "gateway-connection-check";

let ws = null;
let isConnecting = false;
let heartbeatTimer = null;
let reconnectTimer = null;

function log(message, extra) {
  if (extra === undefined) {
    console.log(`[CookieGateway] ${message}`);
    return;
  }
  console.log(`[CookieGateway] ${message}`, extra);
}

function isWebSocketOpen() {
  return ws && ws.readyState === WebSocket.OPEN;
}

function ensureAlarm() {
  chrome.alarms.create(CONNECTION_CHECK_ALARM, { periodInMinutes: 1 });
}

function sendJson(payload) {
  if (!isWebSocketOpen()) {
    return false;
  }

  ws.send(JSON.stringify(payload));
  return true;
}

function stopHeartbeat() {
  if (!heartbeatTimer) {
    return;
  }

  clearInterval(heartbeatTimer);
  heartbeatTimer = null;
}

function startHeartbeat() {
  stopHeartbeat();
  heartbeatTimer = setInterval(() => {
    sendJson({
      type: "PING",
      timestamp: new Date().toISOString(),
    });
  }, HEARTBEAT_INTERVAL_MS);
}

function scheduleReconnect(reason) {
  if (reconnectTimer) {
    return;
  }

  log(`Scheduling reconnect in ${RECONNECT_DELAY_MS}ms`, reason);
  reconnectTimer = setTimeout(() => {
    reconnectTimer = null;
    connectGateway();
  }, RECONNECT_DELAY_MS);
}

function clearReconnectSchedule() {
  if (!reconnectTimer) {
    return;
  }

  clearTimeout(reconnectTimer);
  reconnectTimer = null;
}

function chromeCookiesGetAll(filter) {
  return new Promise((resolve, reject) => {
    chrome.cookies.getAll(filter, (cookies) => {
      if (chrome.runtime.lastError) {
        reject(new Error(chrome.runtime.lastError.message));
        return;
      }
      resolve(cookies || []);
    });
  });
}

function chromeTabsCreate(createProperties) {
  return new Promise((resolve, reject) => {
    chrome.tabs.create(createProperties, (tab) => {
      if (chrome.runtime.lastError) {
        reject(new Error(chrome.runtime.lastError.message));
        return;
      }
      resolve(tab);
    });
  });
}

async function runGetCookies(command) {
  const { domain } = command;
  if (!domain || typeof domain !== "string") {
    throw new Error("GET_COOKIES requires a string field: domain");
  }

  const cookies = await chromeCookiesGetAll({ domain });
  return {
    domain,
    count: cookies.length,
    cookies,
  };
}

async function runOpenUrl(command) {
  const { url } = command;
  if (!url || typeof url !== "string") {
    throw new Error("OPEN_URL requires a string field: url");
  }

  const tab = await chromeTabsCreate({ url, active: true });
  return {
    message: "URL opened",
    tabId: tab?.id ?? null,
    url: tab?.url ?? url,
  };
}

async function handleCommand(command) {
  const requestId = command?.requestId ?? null;
  const action = command?.action;

  try {
    if (!action || typeof action !== "string") {
      throw new Error("Missing action field");
    }

    let result;
    switch (action) {
      case "GET_COOKIES":
        result = await runGetCookies(command);
        break;
      case "OPEN_URL":
        result = await runOpenUrl(command);
        break;
      default:
        throw new Error(`Unsupported action: ${action}`);
    }

    sendJson({
      type: "ACTION_RESULT",
      ok: true,
      requestId,
      action,
      result,
      timestamp: new Date().toISOString(),
    });
  } catch (error) {
    sendJson({
      type: "ACTION_RESULT",
      ok: false,
      requestId,
      action: action || "UNKNOWN",
      error: error instanceof Error ? error.message : String(error),
      timestamp: new Date().toISOString(),
    });
  }
}

function connectGateway() {
  if (isWebSocketOpen() || isConnecting) {
    return;
  }

  isConnecting = true;
  log(`Connecting to ${GATEWAY_WS_URL}`);

  const nextWs = new WebSocket(GATEWAY_WS_URL);
  ws = nextWs;

  nextWs.onopen = () => {
    isConnecting = false;
    clearReconnectSchedule();
    startHeartbeat();

    sendJson({
      type: "CLIENT_HELLO",
      role: "chrome-extension-client",
      userAgent: navigator.userAgent,
      timestamp: new Date().toISOString(),
    });

    log("Gateway connection established");
  };

  nextWs.onmessage = (event) => {
    let message;
    try {
      message = JSON.parse(event.data);
    } catch (error) {
      log("Received non-JSON message", event.data);
      return;
    }

    if (message.type === "PONG") {
      log("Received PONG");
      return;
    }

    if (!message.action) {
      log("Ignoring message without action", message);
      return;
    }

    handleCommand(message);
  };

  nextWs.onerror = (event) => {
    isConnecting = false;
    log("WebSocket error", event);
  };

  nextWs.onclose = (event) => {
    if (ws === nextWs) {
      ws = null;
    }

    isConnecting = false;
    stopHeartbeat();
    log("WebSocket closed", { code: event.code, reason: event.reason });
    scheduleReconnect("socket-closed");
  };
}

function initialize() {
  ensureAlarm();
  connectGateway();
}

chrome.runtime.onInstalled.addListener(() => {
  log("Extension installed");
  initialize();
});

chrome.runtime.onStartup.addListener(() => {
  log("Browser startup detected");
  initialize();
});

chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm.name !== CONNECTION_CHECK_ALARM) {
    return;
  }

  if (!isWebSocketOpen()) {
    log("Alarm detected disconnected socket; reconnecting");
    connectGateway();
  }
});

chrome.runtime.onSuspend.addListener(() => {
  stopHeartbeat();
  if (ws) {
    ws.close();
    ws = null;
  }
});

initialize();
