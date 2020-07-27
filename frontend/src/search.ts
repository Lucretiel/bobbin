/**
 * Script related to running & styling the search bar
 */

import { requireChanged, fetchElementsByIds } from "./common";
import { settings } from "cluster";

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

    threadButton.toggleAttribute("disabled", !isValid);

    if (isValid) {
      threadButton.setAttribute("href", `/thread/${tweetId}`);
    } else {
      threadButton.removeAttribute("href");
    }

    textField.classList.toggle("is-success", !isEmpty && isValid);
    textField.classList.toggle("is-danger", !isEmpty && !isValid);

    iconElement.classList.toggle("fas", !isEmpty);
    iconElement.classList.toggle("fa-check", isValid && !isEmpty);
    iconElement.classList.toggle("fa-times", !isValid && !isEmpty);
    iconElement.style.display = isEmpty ? "none" : "";
  });

  textField.addEventListener("input", (event) => {
    update((event.target as HTMLInputElement).value);
  });

  update((textField as HTMLInputElement).value);
});
