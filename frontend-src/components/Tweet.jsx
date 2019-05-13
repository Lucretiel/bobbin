import React from 'react'
import PropTypes from 'prop-types'

const twitterPromise = new Promise(resolve => {
	window.twttr.ready(twttr => resolve(twttr))
})

export default class EmbeddedTweet extends React.PureComponent {
	static propTypes = {
		tweetId: PropTypes.string.isRequired,
		runner: PropTypes.func.isRequired,
	}

	constructor(props) {
		super(props)

		this.cancel = false
		this.node = null

		this.state = {
			error: null
		}
	}

	componentDidMount() {
		this.props.runner(() => this.cancel ? null : (twitterPromise
			.then(twttr => twttr.widgets.createTweet(this.props.tweetId, this.node, {
				conversation: "none",
				align: "center",
			}))
			.catch(error => {
				this.setState({error: error});
				console.error(error);
			})
		))
	}

	componentWillUnmount() {
		this.cancel = true
	}

	setNode = node => this.node = node

	render() {
		return <div>
			{
				!this.state.error ?
					<div key="tweet-container" className="tweet-container" ref={this.setNode}></div> :
					<div key="tweet-error" className="tweet-error tweet-like">{JSON.stringify(this.state.error)}</div>
			}
		</div>
	}
}
