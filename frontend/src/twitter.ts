/**
 * Entry-point to the vendored twitter module
 */

import './vendor/twitter/widgets.js'

// Note: we could use twitter-for-web here, but it may not work correctly since
// we vendor widgets.js ourselves instead of using a script tag.
interface PreTwitter {
	ready: (callback: (twttr: Twitter) => void) => void;
}

interface Twitter extends PreTwitter {
	widgets: {
		createTweet(
			tweetId: string,
			element: HTMLElement,
			options?: {
				cards?: "hidden" | "visible";
				conversation?: "none" | "all";
				theme?: "light" | "dark";
				linkColor?: string;
				width?: number | "auto";
				align?: "left" | "right" | "center";
				lang?: string;
				dnt?: boolean;
			}
		): Promise<HTMLElement>;
	};
}

declare global {
	interface Window {
		twttr: Twitter;
	}
}

export default window.twttr;
