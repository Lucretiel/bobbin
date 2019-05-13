import React from 'react'

import Tweet from './Tweet'
import promiseRunner from '../promiseChain'

interface TweetListProps {
	tweetIds: string[],
	fullyRendered(isFullyRendered: boolean): void,
}

const TweetList: React.FC<TweetListProps> = ({tweetIds, fullyRendered}) => {
	React.useEffect(() => {
		fullyRendered(true);
	});

	return <ul className="list-unstyled">{
		tweetIds.map((tweetId: string) =>
			<li key={tweetId}>
				<Tweet tweetId={tweetId} done={() => {}}/>
			</li>
		)
	}</ul>
}

export default TweetList;
