import { sleep, getConfig, displayMsg, closeMsg } from "/js/lib.js";

const config = await getConfig();
const wsPort = config.arbitrary_input.websocket_port;
const opts = config.arbitrary_input.client;
const modal = document.getElementById("arbitrary-modal");
const touchscreenHelp = document.getElementById("touchscreen-help-text");
const keyboardHelp = document.getElementById("keyboard-help-text");

let lastX = null;
let lastY = null;
let lastSendTime = null;
let moveTimeout = null;
let moveStartTime = null;
let touchCount = 0;
let moveCount = 0;
let touchTimeout = null;
let startDetectionTimeout = null;
export let detectionActive = false;
// Start arbitrary input mode on long press
window.addEventListener(
  "touchstart",
  () => {
    if (!detectionActive) {
      startDetectionTimeout = setTimeout(
        startInputDetection,
        opts.start_press_duration,
      );
    }
  },
  false,
);
window.addEventListener(
  "touchend",
  () => {
    if (startDetectionTimeout) {
      clearTimeout(startDetectionTimeout);
    }
  },
  false,
);

export async function stopInputDetection() {
  var modal = document.getElementById("arbitrary-modal");
  modal.style.display = "none";
  await websocket.close();
  detectionActive = false;
}

var websocket;
export async function startInputDetection(launchedWithKeyboard) {
  websocket = await new WebSocket(`ws://${location.host}:${wsPort}`);
  websocket.onmessage = handleSocketMessage;
  if (launchedWithKeyboard) {
    touchscreenHelp.style.display = "none";
    keyboardHelp.style.display = "";
  }
  modal.ontouchstart = touchStart;
  modal.ontouchend = touchStop;
  modal.ontouchmove = touchMove;
  modal.onmousemove = mouseMove;
  modal.onmousedown = mouseStart;
  modal.onmouseup = mouseStop;
  modal.style.display = "block";
  detectionActive = true;
}

async function handleSocketMessage(event) {
  const msg = JSON.parse(event.data);
  console.log(msg);
  switch (msg.type) {
    case "Error":
      displayMsg("Error: " + msg.data);
      break;
  }
}

async function longPress() {
  console.log("Sending long press");
  await sendStart();
  await sleep(opts.long_press_duration);
  await sendStop();
}

async function shortPress() {
  console.log("Sending short press");
  await sendStart();
  await sleep(opts.short_press_duration);
  await sendStop();
}

async function sendStart() {
  console.log("Sending input start");
  await websocket.send(JSON.stringify({ Start: null }));
}

async function sendStop() {
  console.log("Sending input stop");
  await websocket.send(JSON.stringify({ Stop: null }));
}

async function touchStart(e) {
  e.preventDefault();
  if (touchTimeout) {
    clearTimeout(touchTimeout);
  }
  console.log(e);
  lastX = e.touches[0].clientX;
  lastY = e.touches[0].clientY;
}

async function touchStop(e) {
  e.preventDefault();
  touchCount += 1;
  console.log(e);
  touchTimeout = setTimeout(detectTouchType, opts.touch_wait);
}

function resetTouch() {
  touchCount = 0;
  moveCount = 0;
  touchTimeout = null;
  moveTimeout = null;
  lastSendTime = null;
  moveStartTime = null;
}

function detectTouchType() {
  console.log("Touch Count: " + touchCount);
  console.log("Move Count: " + moveCount);
  if (moveCount > opts.move_event_cutoff) {
    resetTouch();
    return;
  }
  if (touchCount == 1) {
    shortPress();
  } else if (touchCount == 2) {
    longPress();
  } else if (touchCount == 3) {
    stopInputDetection();
  }
  resetTouch();
}

async function touchMove(e) {
  e.preventDefault();
  var x = e.touches[0].clientX;
  var y = e.touches[0].clientY;
  sendMoveIfDue(x, y);
}

let justUnpaused = false;
async function mouseMove(e) {
  if (pauseSending) {
    return;
  } else {
    e.preventDefault();
  }
  var x = e.clientX;
  var y = e.clientY;
  if (justUnpaused) {
    lastX = x;
    lastY = y;
    justUnpaused = false;
  }
  sendMoveIfDue(x, y);
}

async function sendMoveIfDue(x, y) {
  if (moveCount === 0) {
    moveStartTime = Date.now();
  }
  moveCount += 1;
  if (moveTimeout) {
    clearTimeout(moveTimeout);
  }

  var now = Date.now();
  var timeSinceStart = now - moveStartTime;
  var timeSinceSend = now - lastSendTime;
  if (timeSinceStart >= opts.move_send_wait && !lastSendTime) {
    await moveRelative(x, y);
  } else if (lastSendTime && timeSinceSend >= opts.move_send_wait) {
    await moveRelative(x, y);
  } else {
    // Make sure we always send the final mouse position
    moveTimeout = setTimeout(() => {
      moveRelative(x, y);
    }, opts.final_move_send_delay);
  }
}

async function mouseStart(e) {
  e.preventDefault();
  sendStart();
}

async function mouseStop(e) {
  e.preventDefault();
  sendStop();
}

async function moveRelative(clientX, clientY) {
  var x = 0;
  var y = 0;
  if (lastX != null) {
    x = lastX - clientX;
    y = lastY - clientY;
  }
  x *= opts.sensitivity;
  y *= opts.sensitivity;
  lastX = clientX;
  lastY = clientY;
  await sendMoveRelative(x, y);
}

async function sendMoveRelative(x, y) {
  console.log(`Sending input move relative. X: ${x}, Y: ${y}`);
  let msg = { MoveRelative: { x: -x, y: -y } };
  lastSendTime = Date.now();
  await websocket.send(JSON.stringify(msg));
}

// Keboard Controls
let tapInProgress = false;
let pauseSending = false;
async function handleKeyDown(event) {
  if (!detectionActive) {
    if (event.code === opts.start_shortcut) {
      startInputDetection(true);
    }
    return;
  }
  var moveDistance = opts.arrow_move_distance;
  if (event.getModifierState("Control")) {
    moveDistance = Math.round(moveDistance * opts.control_move_multiplier);
  }
  if (event.getModifierState("Shift")) {
    moveDistance = Math.round(moveDistance * opts.shift_move_multiplier);
  }
  switch (event.code) {
    case "Right":
    case "ArrowRight":
    case "KeyD":
      await sendMoveRelative(-moveDistance, 0);
      break;
    case "Left":
    case "ArrowLeft":
    case "KeyA":
      await sendMoveRelative(moveDistance, 0);
      break;
    case "Up":
    case "ArrowUp":
    case "KeyW":
      await sendMoveRelative(0, moveDistance);
      break;
    case "Down":
    case "ArrowDown":
    case "KeyS":
      await sendMoveRelative(0, -moveDistance);
      break;
    case "KeyQ":
      // Stop detectionActive being set to false before other keydown listeners have run
      await sleep(20);
      stopInputDetection();
      break;
    case "KeyC":
      pauseSending = !pauseSending;
      if (!pauseSending) {
        justUnpaused = true;
      }
      break;
    case "KeyR":
      console.log("Sending refresh");
      await websocket.send('"Reinit"');
      break;
    case "Space":
      if (tapInProgress) {
        return;
      }
      await sendStart();
      tapInProgress = true;
      break;
    case "Escape":
      closeMsg();
      break;
  }
}

async function handleKeyUp(event) {
  switch (event.code) {
    case "Space":
      tapInProgress = false;
      await sendStop();
  }
}

window.addEventListener("keydown", handleKeyDown, false);
window.addEventListener("keyup", handleKeyUp, false);
