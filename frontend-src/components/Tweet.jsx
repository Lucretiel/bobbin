import React from 'react'
import PropTypes from 'prop-types'

export default class EmbeddedTweet extends React.PureComponent {
	static propTypes = {
		tweetId: PropTypes.string.isRequired,
		runner: PropTypes.func.isRequired,
	}

	setNode(node) {
		this.node = node
	}

	componentDidMount() {
		this.props.runner(() =>
			window.twttr.widgets.createTweet(this.props.tweetId, this.node, {
				conversation: "none",
				align: "center",
			})
		)
	}

	render() {
		return <div className="tweet-container" ref={node => this.setNode(node)}></div>
	}
}
