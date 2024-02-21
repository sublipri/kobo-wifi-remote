// SPDX-FileCopyrightText: 2023 sublipri <sublipri@proton.me>
// SPDX-License-Identifier: GPL-3.0-only

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
  console.log(forms);
  for (const form of forms) {
    const button = form.querySelector("button");
    console.log(button);
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
    const result_modal = `
    <div id="result-modal" class="modal">
      <div class="modal-content">
        <span id="result-close-modal" class="close">&times;</span>
        <p></p>
      </div>
    </div>
`;
    document.body.insertAdjacentHTML("beforeend", alert_recording_modal);
    document.body.insertAdjacentHTML("beforeend", result_modal);

    const close = document.getElementById(`${button.id}-close-modal`);
    close.onclick = function () {
      const modal = document.getElementById(`${button.id}-modal`);
      modal.style.display = "none";
      clearInterval(timers[button.id]);
      window.stop();
    };
    const close_result = document.getElementById("result-close-modal");
    close_result.onclick = closeResult;
  }
}

function closeResult() {
  const modal = document.getElementById("result-modal");
  modal.style.display = "none";
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

// Convert HTML form to Javascript object
function processForm(form) {
  var data = {};
  for (const input of form.querySelectorAll("input")) {
    if (input.value === "") {
      continue;
    }
    if (input.className === "input-number") {
      data[input.name] = parseInt(input.value, 10);
    } else if (input.type === "checkbox") {
      data[input.name] = input.checked;
    } else {
      data[input.name] = input.value;
    }
  }
  return data;
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
  const result = document.getElementById("result-modal");
  const p = result.querySelector("p");
  if (response.ok) {
    const recorded = await response.json();
    p.innerHTML = `Recorded ${recorded.name} in ${recorded.rotation} rotation`;
    result.style.display = "block";
    await sleep(2000);
    closeResult();
  } else {
    p.innerHTML = await response.text();
    result.style.display = "block";
  }
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
