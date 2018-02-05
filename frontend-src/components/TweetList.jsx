import React from 'react'
import PropTypes from 'prop-types'

import _ from 'lodash'

import Tweet from 'components/Tweet.jsx'
import promiseRunner from 'promiseChain.jsx'

const cancelError = new Error("Cancelled")

const ifNotCancelled = handler => result => result === cancelError ? result : handler(result)

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
		this.loadingTask = Promise.resolve()
		this.cancelled = false
		this.cancelTask = new Promise((resolve, reject) => {
			this.cancelFunc = () => {
				resolve(cancelError)
				this.cancelled = true
			}
		})
	}

	tweetLoading(tweetLoadedPromise) {
		this.props.fullyRendered(false)

		this.loadingTask = Promise.race([
			// Chain the two completion loaders
			Promise.all([
				// Undo the fullyRendered call from the previous completion, and
				// suppress cancelation rejections from it.
				this.loadingTask.then(ifNotCancelled(result => this.props.fullyRendered(false))),

				//TODO: find a way to cancel tweet loading elegantly
				tweetLoadedPromise.catch(reason => {
					if(!this.cancelled) console.error("Error loading tweet:", reason)
				}),
			]),
			this.cancelTask
		]).then(ifNotCancelled(result => this.props.fullyRendered(true)))
	}

	componentWillUnmount() {
		this.cancelFunc()
	}

	render() {
		return <ul className="list-unstyled">{
			_.map(this.props.tweetIds, tweetId =>
				<li key={tweetId}>
					<Tweet tweetId={tweetId} runner={
						loadTweet => this.tweetLoading(this.runner(loadTweet))
					}/>
				</li>
			)
		}</ul>
	}
}
