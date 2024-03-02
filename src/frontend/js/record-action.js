// SPDX-FileCopyrightText: 2023 sublipri <sublipri@proton.me>
// SPDX-License-Identifier: GPL-3.0-only

import { processForm, displayMsg } from "/js/lib.js";
const timers = {};

function reset_alerts() {
  const buttons = document.getElementsByClassName("records-input");
  for (const button of buttons) {
    const modal = document.getElementById(`${button.id}-modal`);
    modal.style.display = "none";
    clearInterval(timers[button.id]);
  }
}

export function setup() {
  window.addEventListener("pageshow", reset_alerts);
  const forms = document.querySelectorAll(".action-form");
  for (const form of forms) {
    const button = form.querySelector("button");
    button.onclick = function () {
      recordAction(form, button.id);
    };
    const alert_recording_modal = `
    <div id="${button.id}-modal" class="modal">
      <div class="modal-content">
        <span id="${button.id}-close-modal" class="close">&times;</span>
        <p>Recording Input</p>
        <div id="${button.id}-countdown"></div>
      </div>
    </div>
`;
    document.body.insertAdjacentHTML("beforeend", alert_recording_modal);

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
      countdown.innerHTML = "";
    }
  }, 1000);
  timers[id] = timer;
}

export async function recordAction(form, id) {
  if (!form.checkValidity()) {
    form.reportValidity();
    return;
  }
  const data = processForm(form);
  const payload = JSON.stringify(data);
  const config = {
    method: "POST",
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
    },
    body: payload,
  };
  alertRecording(data.no_input_timeout / 1000, id);
  const response = await fetch("/actions", config);
  reset_alerts();
  if (response.ok) {
    const recorded = await response.json();
    displayMsg(
      `Recorded ${recorded.name} in ${recorded.rotation} rotation`,
      1500,
    );
  } else {
    displayMsg(await response.text());
  }
}
