/**
 * Script related to running & styling the search bar
 */

import { whenReady, classNames, requireChanged } from "./common.mjs"

whenReady(() => {
    const tweetRegex = /^\s*(?:(?:https?:\/\/)?(?:(?:www|mobile)\.)?twitter\.com\/\w+\/status\/)?(\d{1,24})(?:[?#]\S*)?\s*$/;

    const textField = document.getElementById("thread-input-field");
    const threadButton = document.getElementById("thread-button");
    const iconElement = document.getElementById("thread-input-icon");

    const extractTweetId = searchText => {
        const match = tweetRegex.exec(searchText);
        return match == null ? null : match[1];
    }

    const update = requireChanged(searchText => {
        const tweetId = extractTweetId(searchText);
        const isEmpty = searchText === "";
        const isValid = tweetId != null;

        const buttonClass = classNames({
            "button": true,
            "is-link": true,
            "transition": true,
        })

        const textInputClass = classNames({
            "input": true,
            "transition": true,
            "is-success": !isEmpty && isValid,
            "is-danger": !isEmpty && !isValid,
        })

        const iconClass = classNames({
            "fas": true,
            "fa-check": isValid,
            "fa-times": !isValid,
        })

        threadButton.className = buttonClass;
        textField.className = textInputClass;

        iconElement.className = iconClass;
        iconElement.style.display = isEmpty ? "none" : "";

        if (isValid) {
            threadButton.setAttribute("href", `/thread/${tweetId}`);
            threadButton.removeAttribute("disabled")
        } else {
            threadButton.removeAttribute("href");
            threadButton.setAttribute("disabled", true)
        }
    });

    textField.addEventListener("input", event => {
        // TODO: filter for text changes only.
        update(event.target.value);
    })

    update(textField.value);
})
