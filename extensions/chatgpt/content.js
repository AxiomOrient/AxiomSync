(function () {
  const ACTION_ID = "axiomsync-selection-action";
  const TOAST_ID = "axiomsync-selection-toast";
  const HINT_WINDOW = 32;

  let currentPacket = null;

  function stableHash(input) {
    let hash = 2166136261;
    for (let i = 0; i < input.length; i += 1) {
      hash ^= input.charCodeAt(i);
      hash = Math.imul(hash, 16777619);
    }
    return `ax-${(hash >>> 0).toString(16)}`;
  }

  function collapseWhitespace(value) {
    return String(value || "").replace(/\s+/g, " ").trim();
  }

  function rawTextFromNode(node) {
    return String(node?.textContent || "");
  }

  function closestMessageBlock(node) {
    const element = node instanceof Element ? node : node?.parentElement;
    return element?.closest?.("[data-message-author-role]") || null;
  }

  function selectionRange() {
    const selection = window.getSelection();
    if (!selection || selection.rangeCount === 0 || selection.isCollapsed) {
      return null;
    }
    return selection.getRangeAt(0);
  }

  function messageRole(block) {
    return block.getAttribute("data-message-author-role") || "user";
  }

  function fallbackMessageId(block, blockText) {
    return stableHash(`${location.pathname}:${messageRole(block)}:${blockText}`);
  }

  function messageId(block, blockText) {
    return block.getAttribute("data-message-id") || fallbackMessageId(block, blockText);
  }

  function messageIndex(block) {
    return Array.from(document.querySelectorAll("[data-message-author-role]")).indexOf(block);
  }

  function domFingerprint(block, blockText) {
    const role = messageRole(block);
    const candidateId = block.getAttribute("data-message-id") || "";
    const tag = block.tagName.toLowerCase();
    const childCount = block.childElementCount;
    return stableHash(`${location.pathname}:${tag}:${role}:${candidateId}:${childCount}:${blockText}`);
  }

  function selectionOffsets(block, range) {
    const startRange = document.createRange();
    startRange.selectNodeContents(block);
    startRange.setEnd(range.startContainer, range.startOffset);

    const endRange = document.createRange();
    endRange.selectNodeContents(block);
    endRange.setEnd(range.endContainer, range.endOffset);

    return {
      start: startRange.toString().length,
      end: endRange.toString().length
    };
  }

  function selectionHints(blockText, offsets) {
    const start = Math.max(0, offsets.start - HINT_WINDOW);
    const end = Math.min(blockText.length, offsets.end + HINT_WINDOW);
    return {
      start_hint: collapseWhitespace(blockText.slice(start, offsets.start)),
      end_hint: collapseWhitespace(blockText.slice(offsets.end, end))
    };
  }

  function buildPacket(range) {
    const startBlock = closestMessageBlock(range.startContainer);
    const endBlock = closestMessageBlock(range.endContainer);
    if (!startBlock || startBlock !== endBlock) {
      return null;
    }

    const selectedTextRaw = range.toString();
    const selectedText = collapseWhitespace(selectedTextRaw);
    if (!selectedText) {
      return null;
    }

    const rawBlockText = rawTextFromNode(startBlock);
    const normalizedBlockText = collapseWhitespace(rawBlockText);
    if (!normalizedBlockText) {
      return null;
    }

    const role = messageRole(startBlock);
    const sourceMessageId = messageId(startBlock, normalizedBlockText);
    const offsets = selectionOffsets(startBlock, range);
    const hints = selectionHints(rawBlockText, offsets);
    const fingerprint = domFingerprint(startBlock, normalizedBlockText);
    const packetId = stableHash([
      location.pathname,
      role,
      sourceMessageId,
      selectedText,
      hints.start_hint,
      hints.end_hint,
      fingerprint
    ].join(":"));
    const capturedAt = Date.now();

    return {
      connector: "chatgpt",
      native_schema_version: "chatgpt-selection-v1",
      native_session_id: location.pathname,
      native_event_id: packetId,
      event_type: "selection_captured",
      ts_ms: capturedAt,
      payload: {
        workspace_root: location.origin,
        turn_id: sourceMessageId,
        actor: role,
        role,
        text: selectedText,
        page_url: location.href,
        page_title: document.title,
        source_message: {
          message_id: sourceMessageId,
          role,
          index: messageIndex(startBlock)
        },
        selection: {
          text: selectedText,
          start_hint: hints.start_hint,
          end_hint: hints.end_hint,
          dom_fingerprint: fingerprint
        },
        captured_at_ms: capturedAt,
        tags: [],
        user_note: null
      }
    };
  }

  function actionButton() {
    let node = document.getElementById(ACTION_ID);
    if (node) {
      return node;
    }
    node = document.createElement("button");
    node.id = ACTION_ID;
    node.type = "button";
    node.textContent = "Send to Axiom";
    Object.assign(node.style, {
      position: "fixed",
      zIndex: "2147483647",
      display: "none",
      padding: "8px 12px",
      borderRadius: "999px",
      border: "1px solid #0f6b5d",
      background: "#0f6b5d",
      color: "#ffffff",
      fontSize: "12px",
      fontWeight: "600",
      cursor: "pointer",
      boxShadow: "0 8px 24px rgba(0,0,0,0.18)"
    });
    node.addEventListener("mousedown", (event) => {
      event.preventDefault();
      event.stopPropagation();
    });
    node.addEventListener("click", () => {
      if (!currentPacket) {
        hideActionButton();
        return;
      }
      chrome.runtime.sendMessage(
        { type: "axiomsync.capture_selection", packet: currentPacket },
        (response) => {
          const error = chrome.runtime.lastError?.message;
          if (error) {
            showToast("Saved locally, retry pending");
          } else if (response?.ok) {
            showToast("Sent to AxiomSync");
          } else if (response?.queued) {
            showToast("Saved locally, retry pending");
          }
        }
      );
      currentPacket = null;
      hideActionButton();
      window.getSelection()?.removeAllRanges();
    });
    document.documentElement.appendChild(node);
    return node;
  }

  function toastNode() {
    let node = document.getElementById(TOAST_ID);
    if (node) {
      return node;
    }
    node = document.createElement("div");
    node.id = TOAST_ID;
    Object.assign(node.style, {
      position: "fixed",
      right: "16px",
      bottom: "16px",
      zIndex: "2147483647",
      display: "none",
      padding: "10px 14px",
      borderRadius: "12px",
      background: "rgba(18, 24, 27, 0.92)",
      color: "#ffffff",
      fontSize: "12px",
      boxShadow: "0 8px 24px rgba(0,0,0,0.22)"
    });
    document.documentElement.appendChild(node);
    return node;
  }

  function showToast(message) {
    const node = toastNode();
    node.textContent = message;
    node.style.display = "block";
    clearTimeout(showToast.timeoutId);
    showToast.timeoutId = setTimeout(() => {
      node.style.display = "none";
    }, 2200);
  }

  function hideActionButton() {
    actionButton().style.display = "none";
  }

  function showActionButton(rect) {
    const button = actionButton();
    const top = Math.max(12, rect.top - 44);
    const left = Math.min(window.innerWidth - 140, Math.max(12, rect.left));
    button.style.top = `${top}px`;
    button.style.left = `${left}px`;
    button.style.display = "block";
  }

  function refreshSelectionAction() {
    const range = selectionRange();
    if (!range) {
      currentPacket = null;
      hideActionButton();
      return;
    }

    const packet = buildPacket(range);
    if (!packet) {
      currentPacket = null;
      hideActionButton();
      return;
    }

    const rect = range.getBoundingClientRect();
    if (!rect || (!rect.width && !rect.height)) {
      currentPacket = null;
      hideActionButton();
      return;
    }

    currentPacket = packet;
    showActionButton(rect);
  }

  document.addEventListener("selectionchange", refreshSelectionAction);
  window.addEventListener("resize", refreshSelectionAction);
  document.addEventListener("scroll", refreshSelectionAction, true);
  document.addEventListener("mousedown", (event) => {
    const button = document.getElementById(ACTION_ID);
    if (button && !button.contains(event.target)) {
      hideActionButton();
    }
  });
})();
