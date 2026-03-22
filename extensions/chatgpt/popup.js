const DEFAULT_ENDPOINT = "http://127.0.0.1:4402/";
const STORAGE_KEYS = {
  endpoint: "endpoint",
  defaultTags: "default_tags",
  pendingQueue: "pending_queue",
  lastDelivery: "last_delivery",
  lastError: "last_error"
};

function tagsToText(tags) {
  if (!Array.isArray(tags) || !tags.length) {
    return "";
  }
  return tags.join(", ");
}

function parseTags(value) {
  return String(value || "")
    .split(",")
    .map((tag) => tag.trim())
    .filter(Boolean);
}

function formatTime(value) {
  if (!value?.at_ms) {
    return "none";
  }
  return new Date(value.at_ms).toLocaleString();
}

async function loadState() {
  const state = await chrome.storage.local.get({
    [STORAGE_KEYS.endpoint]: DEFAULT_ENDPOINT,
    [STORAGE_KEYS.defaultTags]: [],
    [STORAGE_KEYS.pendingQueue]: [],
    [STORAGE_KEYS.lastDelivery]: null,
    [STORAGE_KEYS.lastError]: null
  });

  document.getElementById("endpoint").value = state[STORAGE_KEYS.endpoint];
  document.getElementById("tags").value = tagsToText(state[STORAGE_KEYS.defaultTags]);
  document.getElementById("queue-count").textContent = String(
    state[STORAGE_KEYS.pendingQueue].length
  );
  document.getElementById("last-delivery").textContent = formatTime(
    state[STORAGE_KEYS.lastDelivery]
  );
  document.getElementById("last-error").textContent =
    state[STORAGE_KEYS.lastError]?.message || "none";
}

async function saveState() {
  const endpoint = document.getElementById("endpoint").value.trim() || DEFAULT_ENDPOINT;
  const tags = parseTags(document.getElementById("tags").value);

  await chrome.storage.local.set({
    [STORAGE_KEYS.endpoint]: endpoint,
    [STORAGE_KEYS.defaultTags]: tags
  });

  await loadState();
}

document.getElementById("save").addEventListener("click", () => {
  saveState().catch((error) => {
    document.getElementById("last-error").textContent = String(error);
  });
});

chrome.storage.onChanged.addListener(() => {
  loadState().catch(() => {});
});

loadState().catch(() => {});
