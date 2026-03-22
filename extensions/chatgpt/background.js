const DEFAULT_ENDPOINT = "http://127.0.0.1:4402/";
const RETRY_ALARM = "axiomsync-retry";
const RETRY_PERIOD_MINUTES = 1;

const STORAGE_KEYS = {
  endpoint: "endpoint",
  defaultTags: "default_tags",
  pendingQueue: "pending_queue",
  lastDelivery: "last_delivery",
  lastError: "last_error"
};

function normalizeEndpoint(value) {
  const raw = typeof value === "string" ? value.trim() : "";
  if (!raw) {
    return DEFAULT_ENDPOINT;
  }
  return raw.endsWith("/") ? raw : `${raw}/`;
}

function normalizeTags(value) {
  if (!Array.isArray(value)) {
    return [];
  }
  return value
    .map((tag) => (typeof tag === "string" ? tag.trim() : ""))
    .filter(Boolean);
}

function packetId(packet) {
  return packet?.native_event_id || "";
}

function queueWithoutDuplicate(queue, packet) {
  const id = packetId(packet);
  if (!id) {
    return queue.concat([packet]);
  }
  if (queue.some((candidate) => packetId(candidate) === id)) {
    return queue;
  }
  return queue.concat([packet]);
}

function enrichPacket(packet, defaultTags) {
  const payload = packet?.payload || {};
  const tags = payload.tags?.length ? payload.tags : defaultTags;
  return {
    ...packet,
    payload: {
      ...payload,
      tags
    }
  };
}

function deliveryPlan(settings, packet) {
  return {
    endpoint: normalizeEndpoint(settings[STORAGE_KEYS.endpoint]),
    packet: enrichPacket(packet, normalizeTags(settings[STORAGE_KEYS.defaultTags]))
  };
}

async function postPacket(endpoint, packet) {
  const response = await fetch(endpoint, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ events: [packet] })
  });
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}`);
  }
}

async function loadSettings() {
  return chrome.storage.local.get({
    [STORAGE_KEYS.endpoint]: DEFAULT_ENDPOINT,
    [STORAGE_KEYS.defaultTags]: [],
    [STORAGE_KEYS.pendingQueue]: [],
    [STORAGE_KEYS.lastDelivery]: null,
    [STORAGE_KEYS.lastError]: null
  });
}

async function rememberDelivery(packet) {
  await chrome.storage.local.set({
    [STORAGE_KEYS.lastDelivery]: {
      at_ms: Date.now(),
      packet_id: packetId(packet)
    },
    [STORAGE_KEYS.lastError]: null
  });
}

async function rememberError(error) {
  await chrome.storage.local.set({
    [STORAGE_KEYS.lastError]: {
      at_ms: Date.now(),
      message: String(error)
    }
  });
}

async function enqueuePacket(packet) {
  const settings = await loadSettings();
  const nextQueue = queueWithoutDuplicate(settings[STORAGE_KEYS.pendingQueue], packet);
  await chrome.storage.local.set({
    [STORAGE_KEYS.pendingQueue]: nextQueue
  });
  return nextQueue.length;
}

async function deliverPacket(packet) {
  const settings = await loadSettings();
  const plan = deliveryPlan(settings, packet);
  await postPacket(plan.endpoint, plan.packet);
  await rememberDelivery(plan.packet);
  return plan.packet;
}

async function captureSelection(packet) {
  try {
    await deliverPacket(packet);
    return { ok: true, queued: false };
  } catch (error) {
    await enqueuePacket(packet);
    await rememberError(error);
    return { ok: false, queued: true, error: String(error) };
  }
}

async function retryPendingQueue() {
  const settings = await loadSettings();
  const queue = settings[STORAGE_KEYS.pendingQueue];
  if (!queue.length) {
    return;
  }

  const delivered = [];
  for (const packet of queue) {
    try {
      await deliverPacket(packet);
      delivered.push(packetId(packet));
    } catch (error) {
      await rememberError(error);
      break;
    }
  }

  if (!delivered.length) {
    return;
  }

  const remaining = queue.filter((packet) => !delivered.includes(packetId(packet)));
  await chrome.storage.local.set({
    [STORAGE_KEYS.pendingQueue]: remaining
  });
}

function scheduleRetryAlarm() {
  chrome.alarms.create(RETRY_ALARM, {
    periodInMinutes: RETRY_PERIOD_MINUTES
  });
}

chrome.runtime.onInstalled.addListener(scheduleRetryAlarm);
chrome.runtime.onStartup.addListener(scheduleRetryAlarm);
scheduleRetryAlarm();

chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm.name !== RETRY_ALARM) {
    return;
  }
  retryPendingQueue().catch(() => {});
});

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message?.type !== "axiomsync.capture_selection" || !message.packet) {
    return false;
  }

  captureSelection(message.packet)
    .then(sendResponse)
    .catch((error) => sendResponse({ ok: false, queued: false, error: String(error) }));

  return true;
});
