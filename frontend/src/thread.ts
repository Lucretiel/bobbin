import { fetchElementsByClass, fetchElementById } from "./common";
import { error, time } from "console";
import { settings } from "cluster";

// This is adapted from twitter's recommended installation script:
// https://developer.twitter.com/en/docs/twitter-for-websites/javascript-api/guides/set-up-twitter-for-websites
//
// It returns a promise that resolves to the `Twitter` object once it's loaded.
const twitter_widgets: () => Promise<Twitter> = () =>
  new Promise((resolve) => {
    let twt = window.twttr;

    if (!twt) {
      const newTwttr: { _e: Array<any> } & TwitterLike = {
        _e: [],
        ready: (f) => newTwttr._e.push(f),
      };

      twt = window.twttr = newTwttr;
    }

    twt.ready((twttr) => resolve(twttr));
  });

// Wrapper for twttr.widgets.createTweet that throws an error in the event of
// a failure.
const createThreadItem = (
  tweet_id: string,
  element: HTMLElement,
  options?: TwitterTweetWidgetOptions
) =>
  twitter_widgets()
    .then((twttr) => twttr.widgets.createTweet(tweet_id, element, options))
    .then((e) => {
      if (e == null) {
        throw new Error("Failed to load tweet");
      } else {
        return e;
      }
    });

Promise.all([
  fetchElementsByClass("tweet-container"),
  fetchElementById("thread-end-message"),
]).then(([tweet_containers, end_element]) => {
  // For each tweet container, use twttr.widgets to create a tweet widget
  // inside of it. If that fails, show an error in that slot. Additionally,
  // immediately hide the event; we will later unhide them in order to
  // prevent out-of-order rendering. tweet_tasks is an ordered list of
  // {element, loadTask} where each task resolves when that item is finished.
  const tweet_tasks = tweet_containers.map((element) => {
    const tweet_id = element.attributes.getNamedItem("data-tweet-id")?.value;
    if (tweet_id == null)
      throw new Error("Tweet container didn't have a data-tweet-id attribute");

    const loadTask = createThreadItem(tweet_id, element, {
      align: "center",
      conversation: "none",
    })
      .catch((e) => {
        // There exists a pre-rendered but hidden element that indicates an
        // error loading the tweet. If there was *actually* an error loading
        // the tweet, unhide it.
        Array.from(element.getElementsByClassName("tweet-failure")).forEach(
          (errorElement) => {
            errorElement.classList.remove("hidden");
          }
        );
      })
      .then(() => {
        // Once the tweet has loaded, immediately hide the container. We can't
        // have it be hidden before the tweet loads; for some reason, the
        // twitter widgets API does work properly when you load inside a
        // display:hidden
        element.classList.add("hidden");
      });

    return { element, loadTask };
  });

  // tweet_tasks is an ordered list of Promises, each of which resolves when
  // that item is ready (either the tweet loaded or an error occurred). Set
  // up a chain to ensure that each tweet is displayed in order (only unhide
  // them when all the previous ones have also been unhidden).
  const threadTask = tweet_tasks.reduce(
    (chain, { element, loadTask }) =>
      chain
        .then(() => loadTask)
        .then(() => {
          element.classList.remove("hidden");
        }),

    Promise.resolve()
  );

  // The end_element says "Loading Tweets..." when the page loads. Change it to
  // End Of Thread when all the tweets are done.
  threadTask.then(() => {
    end_element.innerText = "End of Thread";
  });
});
