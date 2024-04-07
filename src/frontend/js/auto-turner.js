import { playAction } from "/js/lib.js";

const toggle_btn = document.getElementById("toggle-auto-turner");
const pause_btn = document.getElementById("pause-auto-turner");
const next_btn = document.getElementById("next-page");
const prev_btn = document.getElementById("prev-page");
let interval = null;
let counter = document.getElementById("next-turn-value");
let isPaused = false;
let timeUntilNext = document.getElementById("page-turn-delay").value;

toggle_btn.onclick = start;
pause_btn.onclick = pause;
next_btn.onclick = async () => {
  await playAction("next-page");
};
prev_btn.onclick = async () => {
  await playAction("prev-page");
};

function pause() {
  isPaused = true;
  pause_btn.onclick = resume;
  pause_btn.innerHTML = "Resume";
}

function resume() {
  isPaused = false;
  pause_btn.onclick = pause;
  pause_btn.innerHTML = "Pause";
}

async function start() {
  resetCounter();
  interval = setInterval(handleTick, 1000);
  toggle_btn.innerHTML = "Stop";
  toggle_btn.onclick = stop;
}

async function stop() {
  clearInterval(interval);
  toggle_btn.innerHTML = "Start";
  toggle_btn.onclick = start;
  resetCounter();
}

function resetCounter() {
  timeUntilNext = document.getElementById("page-turn-delay").value;
  counter.innerHTML = timeUntilNext;
}

async function handleTick() {
  if (isPaused) {
    return;
  }
  timeUntilNext -= 1;
  if (timeUntilNext < 1) {
    await playAction("next-page");
    resetCounter();
  } else {
    counter.innerHTML = timeUntilNext;
  }
}
