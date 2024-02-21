// SPDX-FileCopyrightText: 2023 sublipri <sublipri@proton.me>
// SPDX-License-Identifier: GPL-3.0-only

var buttons = document.getElementsByClassName("colored-button");
for (const button of buttons) {
  button.addEventListener("click", function () {
    const regexpEmojiPresentation = /\p{Emoji_Presentation}/gu;
    if (button.innerText.match(regexpEmojiPresentation)) {
      button.style.fontSize = "32px";
      setTimeout(function () {
        button.style.fontSize = "24px";
      }, 100);
    } else {
      button.style.color = "black";
      setTimeout(function () {
        button.style.color = null;
      }, 100);
    }
  });
}
