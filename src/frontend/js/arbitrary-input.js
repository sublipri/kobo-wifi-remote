import { sleep, getConfig, displayMsg, closeMsg } from "/js/lib.js";

const config = await getConfig();
const wsPort = config.user.arbitrary_input.websocket_port;
const opts = config.user.arbitrary_input.client;
const modal = document.getElementById("arbitrary-modal");
const touchscreenHelp = document.getElementById("touchscreen-help-text");
const keyboardHelp = document.getElementById("keyboard-help-text");

let startX = null;
let startY = null;
let lastMoveX = null;
let lastMoveY = null;
let lastSentX = null;
let lastSentY = null;
let touchStartTime = null;
let lastSendTime = null;
let moveTimeout = null;
let moveStartTime = null;
let tapCount = 0;
let moveCount = 0;
let touchTimeout = null;
let startLongpressTimeout = null;
var websocket;
export let detectionActive = false;
// Listeners that handle starting arbitrary input mode
window.addEventListener(
  "touchstart",
  (e) => {
    if (detectionActive) {
      return;
    }
    startX = e.touches[0].clientX;
    startY = e.touches[0].clientY;
    lastMoveX = startX;
    lastMoveY = startY;
    if (opts.start_on_longpress) {
      startLongpressTimeout = setTimeout(
        startInputDetection,
        opts.start_press_duration,
      );
    }
  },
  false,
);
window.addEventListener(
  "touchend",
  async () => {
    if (startLongpressTimeout) {
      clearTimeout(startLongpressTimeout);
    }
    if (detectionActive) {
      return;
    }
    if (opts.start_on_swipe) {
      let min = opts.start_swipe_min_distance;
      let diffX = startX - lastMoveX;
      let diffY = startY - lastMoveY;
      let swipeWasFarEnough =
        diffX > min || diffX < -min || diffY > min || diffY < -min;
      if (swipeWasFarEnough) {
        startInputDetection();
        while (true) {
          await sleep(10);
          if (!websocket) {
            continue;
          } else if (websocket.readyState === 1) {
            await sendMoveRelative(diffX, diffY);
            break;
          }
        }
      }
      resetTouch();
    }
  },
  false,
);

window.addEventListener(
  "touchmove",
  (e) => {
    if (detectionActive) {
      return;
    }
    if (opts.start_on_swipe && opts.swipe_prevent_default) {
      e.preventDefault();
    }
    lastMoveX = e.touches[0].clientX;
    lastMoveY = e.touches[0].clientY;
  },
  { passive: false, capture: false },
);

export async function stopInputDetection() {
  var modal = document.getElementById("arbitrary-modal");
  modal.style.display = "none";
  await websocket.close();
  websocket = null;
  detectionActive = false;
}

export async function startInputDetection(launchedWithKeyboard) {
  console.log("Starting input detection");
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
  startX = e.touches[0].clientX;
  startY = e.touches[0].clientY;
  lastMoveX = startX;
  lastMoveY = startY;
  lastSentX = startX;
  lastSentY = startY;
  touchStartTime = Date.now();
}

async function touchStop(e) {
  e.preventDefault();
  console.log(e);
  let touchDuration = Date.now() - touchStartTime;
  let max = opts.tap_distance_cutoff;
  let min = -max;
  let diffX = lastMoveX - startX;
  let diffY = lastMoveY - startY;
  let touchWasTap =
    touchDuration < 500 &&
    diffX < max &&
    diffX > min &&
    diffY < max &&
    diffY > min;
  console.log("touchWasTap: " + touchWasTap);
  if (touchWasTap) {
    tapCount += 1;
  } else {
    resetTouch();
  }
  touchTimeout = setTimeout(handleTouchTap, opts.touch_wait_duration);
}

function resetTouch() {
  if (touchTimeout) {
    clearTimeout(touchTimeout);
  }
  tapCount = 0;
  moveCount = 0;
  touchTimeout = null;
  moveTimeout = null;
  lastSendTime = null;
  moveStartTime = null;
}

function handleTouchTap() {
  console.log("Tap Count: " + tapCount);
  console.log("Move Count: " + moveCount);
  if (tapCount == 1) {
    shortPress();
  } else if (tapCount == 2) {
    longPress();
  } else if (tapCount == 3) {
    stopInputDetection();
  }
  resetTouch();
}

async function touchMove(e) {
  e.preventDefault();
  var x = e.touches[0].clientX;
  var y = e.touches[0].clientY;
  sendMoveIfDue(x, y);
  lastMoveX = x;
  lastMoveY = y;
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
    lastSentX = x;
    lastSentY = y;
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
    await sendMoveRelativeFromAbsolute(x, y);
  } else if (lastSendTime && timeSinceSend >= opts.move_send_wait) {
    await sendMoveRelativeFromAbsolute(x, y);
  } else {
    // Make sure we always send the final mouse position
    moveTimeout = setTimeout(() => {
      sendMoveRelativeFromAbsolute(x, y);
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

async function sendMoveRelativeFromAbsolute(clientX, clientY) {
  var x = 0;
  var y = 0;
  if (lastSentX != null) {
    x = lastSentX - clientX;
    y = lastSentY - clientY;
  }
  x *= opts.sensitivity;
  y *= opts.sensitivity;
  lastSentX = clientX;
  lastSentY = clientY;
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
