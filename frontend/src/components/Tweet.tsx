import React from 'react'

interface Twitter {
	widgets: {
		createTweet(
			tweetId: string,
			element: HTMLElement,
			options?: {
				cards?: 'hidden',
				conversation?: 'none',
				theme?: 'light' | 'dark',
				linkColor?: string,
				width?: number,
				align?: 'left' | 'right' | 'center',
				lang?: string,
				dnt?: boolean,
			}): Promise<HTMLElement>,
	}
}

declare global {
	interface Window {
		twttr: {
			ready(callback: (twttr: Twitter) => void): void,
		}
	}
}
const twitterPromise: Promise<Twitter> = new Promise(resolve => {
	window.twttr.ready(twttr => resolve(twttr))
})

interface TweetProps {
	tweetId: string,
	done: () => void,
}

const EmbeddedTweet: React.FC<TweetProps> = ({tweetId, done}) => {
	const [error, setError] = React.useState<Error | null>(null);
	const [node, setNode] = React.useState<HTMLDivElement | null>(null);

	React.useEffect(() => {
		const loadTweet = async () => {
			const twttr = await twitterPromise;
			if (node === null) {
				return
			} else try {
				await twttr.widgets.createTweet(tweetId, node, {
					conversation: "none",
					align: "center"
				})
			} catch(error) {
				console.error(error);
				setError(error);
			}
		};

		loadTweet().finally(done);
	},

	// We explicitly omit the `done` function from the dependencies list, since the only thing that
	// should trigger a re-render is a new node.
	[node])

	// We us key={tweetId} here to force react to create a new div if the tweetId changes. This
	// will force our effect to re-run.
	return error === null ?
		<div key={tweetId}><div key="tweet-container" className="tweet-container" ref={setNode}></div></div> :
		<div key={tweetId}><div key="tweet-error" className="tweet-error tweet-like">{JSON.stringify(error)}</div></div>
};

export default EmbeddedTweet;
