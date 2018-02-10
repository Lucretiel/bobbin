import React from 'react'
import PropTypes from 'prop-types'

import _ from 'lodash'

import Tweet from 'components/Tweet.jsx'
import promiseRunner from 'promiseChain.jsx'

const xor = (a, b) => (a && !b) || (b && !a)

export default class TweetList extends React.PureComponent {
	static propTypes = {
		tweetIds: PropTypes.arrayOf(
			PropTypes.string.isRequired
		).isRequired,
		fullyRendered: PropTypes.func.isRequired,
	}

	constructor(props) {
		super(props)

		this.runner = promiseRunner(4)
		this.loadingCount = 0
	}

	updateLoadingCount = delta => {
		const prevCount = this.loadingCount
		const newCount = prevCount + delta
		if(xor(newCount, prevCount)) {
			this.props.fullyRendered(newCount === 0)
		}
		this.loadingCount = newCount
	}

	incrementLoading = (delta = 1) => this.updateLoadingCount(delta)
	decrementLoading = (delta = 1) => this.updateLoadingCount(-delta)

	scheduleLoad = loadTweet => this.runner(() => {
		this.incrementLoading()
		return loadTweet()
	}).then(() => this.decrementLoading())

	render() {
		return <ul className="list-unstyled">{
			_.map(this.props.tweetIds, tweetId =>
				<li key={tweetId}>
					<Tweet tweetId={tweetId} runner={this.scheduleLoad}/>
				</li>
			)
		}</ul>
	}
}
