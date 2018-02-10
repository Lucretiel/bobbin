import React from 'react'
import PropTypes from 'prop-types'

import TweetList from 'components/TweetList.jsx'
import Title from 'components/Title.jsx'

export default class ThreadPage extends React.PureComponent {
	static propTypes = {
		head: PropTypes.string,
		tail: PropTypes.string.isRequired,
	}

	constructor(props) {
		super(props)

		this.state = {
			threadTweetIds: null,
			author: null,
			fullyRendered: false,
		}
	}

	componentDidMount() {
		const {head, tail} = this.props

		const query = head ?
			`head=${head}&tail=${tail}` :
			`tail=${tail}`

		fetch(`/api/thread?${query}`)
		.then(response => response.json())
		.then(content => this.setState({
				threadTweetIds: content.thread,
				author: content.author,
		}))
	}

	fullyRenderedCb = rendered => this.setState({
		fullyRendered: rendered
	})

	render() {
		const {threadTweetIds, author, fullyRendered} = this.state

		const header = author ?
			<h3 className="author-header">Thread by <a
				href={`https://twitter.com/${author.handle}`}
				target="_blank">
				<span className="author">
					<span className="author-name">{author.name}</span>{' '}
					<span className="author-handle">@{author.handle}</span>
				</span>
			</a></h3>:
			<h3>Conversation</h3>

		return <div className="container">
			<Title>{
				author ? `Thread by @${author.handle}` :
				threadTweetIds ? "Conversation" :
				"Thread"
			}</Title>
			<div className="row">
				<div className="col text-center">
					{header}
				</div>
			</div>
			<div className="row justify-content-center">
				<div className="col">
					{threadTweetIds === null ?
						null :
						<TweetList
							tweetIds={threadTweetIds}
							fullyRendered={this.fullyRenderedCb}
						/>
					}
				</div>
			</div>
			<div className="row">
				<div className="col">
					<div className="text-center thread-end tweet-like">
						{fullyRendered ?
							<span className="strike">
								<span>End of Thread</span>
							</span> :
							"Loading Tweets..."
						}
					</div>
				</div>
			</div>
		</div>
	}
}
