/**
 * Script related to running & styling the search bar
 */

import { classNames, requireChanged, fetchElementsByIds } from "./common";

const tweetRegex = /^\s*(?:(?:https?:\/\/)?(?:(?:www|mobile)\.)?twitter\.com\/\w+\/status\/)?(\d{1,24})(?:[?#]\S*)?\s*$/;

const extractTweetId = (searchText: string) => {
  const match = tweetRegex.exec(searchText);
  return match == null ? null : match[1];
};

fetchElementsByIds(
  "thread-input-field",
  "thread-button",
  "thread-input-icon"
).then(([textField, threadButton, iconElement]) => {
  const update = requireChanged((searchText: string) => {
    const tweetId = extractTweetId(searchText);
    const isEmpty = searchText === "";
    const isValid = tweetId != null;

    const buttonClass = classNames({
      button: true,
      "is-link": true,
      transition: true,
    });

    const textInputClass = classNames({
      input: true,
      transition: true,
      "is-success": !isEmpty && isValid,
      "is-danger": !isEmpty && !isValid,
    });

    const iconClass = classNames({
      fas: true,
      "fa-check": isValid,
      "fa-times": !isValid,
    });

    threadButton.className = buttonClass;
    textField.className = textInputClass;
    iconElement.className = iconClass;
    iconElement.style.display = isEmpty ? "none" : "";

    if (isValid) {
      threadButton.setAttribute("href", `/thread/${tweetId}`);
      threadButton.removeAttribute("disabled");
    } else {
      threadButton.removeAttribute("href");
      threadButton.setAttribute("disabled", "");
    }
  });

  textField.addEventListener("input", (event) => {
    update((event.target as HTMLInputElement).value);
  });

  update((textField as HTMLInputElement).value);
});
