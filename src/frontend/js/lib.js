// SPDX-FileCopyrightText: 2023 sublipri <sublipri@proton.me>
// SPDX-License-Identifier: GPL-3.0-only

export async function playAction(path_segment) {
  var response;
  try {
    response = await fetch(`/actions/${path_segment}`, {
      signal: AbortSignal.timeout(5000),
    });
  } catch (err) {
    if (err.name === "TimeoutError") {
      displayMsg(`Request to play ${path_segment} timed out after 5 seconds `);
    } else {
      console.error(`Error: type: ${err.name}, message: ${err.message}`);
    }
    return;
  }
  if (!response.ok) {
    displayMsg(await response.text());
  }
}

export async function displayMsg(msg, timeout) {
  let msg_modal = document.getElementById("msg-modal");
  if (!msg_modal) {
    document.body.insertAdjacentHTML(
      "beforeend",
      `
    <div id="msg-modal" class="modal">
      <div class="modal-content">
        <span id="msg-close-modal" class="close">&times;</span>
        <p></p>
      </div>
    </div>
`,
    );
    msg_modal = document.getElementById("msg-modal");
    document.getElementById("msg-close-modal").onclick = closeMsg;
  }

  const p = msg_modal.querySelector("p");
  p.innerHTML = msg;
  msg_modal.style.display = "block";
  if (timeout) {
    await sleep(timeout);
    closeMsg();
  }
}

export function closeMsg() {
  const modal = document.getElementById("msg-modal");
  const p = modal.querySelector("p");
  p.innerHTML = "";
  modal.style.display = "none";
}

export function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

// Convert HTML form to Javascript object
export function processForm(form) {
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

export async function getConfig() {
  const response = await fetch("/config");
  if (response.ok) {
    return await response.json();
  } else {
    displayMsg(await response.text());
  }
}

export async function changeSetting(section, key, value) {
  console.log(value);
  const request = {
    method: "POST",
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
    },
    body: JSON.stringify([
      {
        section: section,
        key: key,
        value: value,
      },
    ]),
  };
  const response = await fetch("/kobo-config", request);
  if (response.ok) {
    console.log(response);
    location.reload();
  } else {
    displayMsg(await response.text());
  }
}
