import React from 'react'
import PropTypes from 'prop-types'

export default class EmbeddedTweet extends React.PureComponent {
	static propTypes = {
		tweetId: PropTypes.string.isRequired,
		runner: PropTypes.func.isRequired,
	}

	state = {
		loaded: false
	}

	setNode(node) {
		this.node = node
	}

	loadingFinished() {
		this.setState({loaded: true})
	}

	componentDidMount() {
		this.props.runner(() =>
			window.twttr.widgets.createTweet(this.props.tweetId, this.node, {
				conversation: "none",
				align: "center",
			}).then(() => this.loadingFinished())
		)
	}

	render() {
		return <div className="outer-tweet">
			{this.state.loaded ? null :
				<div className="tweet-container-placeholder" key="placeholder">
					Loading tweet...
				</div>
			}
			<div
				className="tweet-container"
				visible={this.state.loaded}
				ref={node => this.setNode(node)}
				key="tweet-container"
			/>
		</div>
	}
}
