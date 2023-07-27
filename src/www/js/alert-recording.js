// SPDX-FileCopyrightText: 2023 sublipri <sublipri@proton.me>
// SPDX-License-Identifier: GPL-3.0-only

const timers = {};

function reset() {
  const buttons = document.getElementsByClassName("records-input");
  for (const button of buttons) {
    const modal = document.getElementById(`${button.id}-modal`);
    modal.style.display = "none";
    clearInterval(timers[button.id]);
  }
}

export function setup(duration = 5) {
  window.addEventListener("pageshow", reset);

  const buttons = document.getElementsByClassName("records-input");
  for (const button of buttons) {
    button.onclick = function () {
      alertRecording(duration, button.id);
    };
    const modal_html = `
    <div id="${button.id}-modal" class="modal">
      <div class="modal-content">
        <span id="${button.id}-close-modal" class="close">&times;</span>
        <p>Recording Input</p>
        <div id="${button.id}-countdown"></div>
      </div>
    </div>
`;
    document.body.insertAdjacentHTML("beforeend", modal_html);

    const close = document.getElementById(`${button.id}-close-modal`);
    close.onclick = function () {
      const modal = document.getElementById(`${button.id}-modal`);
      modal.style.display = "none";
      clearInterval(timers[button.id]);
      window.stop();
    };
  }
}

export function alertRecording(timeleft, id) {
  const modal = document.getElementById(`${id}-modal`);
  const countdown = document.getElementById(`${id}-countdown`);
  countdown.innerHTML = timeleft;
  modal.style.display = "block";
  const timer = setInterval(function () {
    timeleft -= 1;
    countdown.innerHTML = timeleft;
    if (timeleft <= 0) {
      clearInterval(timer);
      modal.style.display = "none";
    }
  }, 1000);
  timers[id] = timer;
}
