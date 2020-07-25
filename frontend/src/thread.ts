import { pageReady } from "./common";

// This is adapted from twitter's recommended installation script:
// https://developer.twitter.com/en/docs/twitter-for-websites/javascript-api/guides/set-up-twitter-for-websites
//
// It returns a promise that resolves to the `Twitter` object once it's loaded
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

const createThreadItem = (
  twttr: Twitter,
  tweet_id: string,
  element: HTMLElement
) =>
  twttr.widgets
    .createTweet(tweet_id, element, {
      align: "center",
      conversation: "none",
    })
    .then((e) => {
      if (e == null) throw new Error("Failed to load tweet");
    });

Promise.all([pageReady(), twitter_widgets()]).then(([_, twttr]) => {
  const tweet_containers = Array.from(
    document.getElementsByClassName("tweet-container")
  ) as Array<HTMLElement>;

  const end_element = document.getElementById("thread-end-message");
  if (end_element == null) throw new Error("No thread-end-message element");

  const tweet_tasks = tweet_containers.map((element) => {
    const tweet_id = element.attributes.getNamedItem("data-tweet-id")?.value;
    if (tweet_id == null)
      throw new Error("Tweet container didn't have a data-tweet-id attribute");

    return createThreadItem(twttr, tweet_id, element)
      .then((e) => console.log("Rendered tweet"))
      .catch((e) => console.error("Failed tweet", tweet_id));
  });

  Promise.all(tweet_tasks).then(() => {
    end_element.innerText = "End of Thread";
  });
});
