import React from 'react'
import PropTypes from 'prop-types'
import classNames from 'classnames'

import TweetList from 'components/TweetList.jsx'
import Title from 'components/Title.jsx'

const tweetRegex = /^\s*(?:(?:https?:\/\/)?(?:(?:www|mobile)\.)?twitter\.com\/[a-zA-Z0-9_]{1,15}\/status\/)?([0-9]{1,24})(?:[?#]\S*)?\s*$/

const getTweetId = tweetLink => {
	const match = tweetRegex.exec(tweetLink)
	return match === null ? null : match[1]
}

class TweetEntryForm extends React.PureComponent {
	static propTypes = {
		submit: PropTypes.func.isRequired,
	}

	constructor(props) {
		super(props)

		this.state = {
			tweetLink: "",
			tweetId: null,
		}
	}

	setLink = event => this.setState({
		tweetLink: event.target.value,
		tweetId: getTweetId(event.target.value),
	})

	submitId = () => {
		const { tweetId } = this.state
		if(tweetId) {
			this.props.submit(tweetId)
		}
	}

	render() {
		const formText = this.state.tweetLink
		const tweetId = this.state.tweetId

		const isEmpty = formText === ""
		const isValid = tweetId !== null

		const submitClass = classNames(
			"btn", {
				"btn-primary": isValid,
				"btn-outline-primary": !isValid,
				disabled: !isValid,
		})

		const textInputClass = classNames(
			"form-control-lg",
			"form-control", {
				"is-valid": !isEmpty && isValid,
				"is-invalid": !isEmpty && !isValid
		})

		return <form id="tweet-entry-form">
			<div className="form-row">
				<div className="col">
					<div className="form-group">
						<input
							type="text"
							className={textInputClass}
							placeholder="Link to last tweet in thread"
							value={formText}
							onChange={this.setLink}
						/>
					</div>
				</div>
			</div>
			<div className="form-row justify-content-center">
				<div className="col col-md-7 col-sm-9">
					<div className="collapse" id="submit-help-block">
						<p><small>
							To view a twitter thread, find the last tweet in the thread you
							want to view, and copy-paste a link to the tweet above. Bobbin
							will automatically follow the reply-chain backwards to the
							beginning of the thread, and display the whole thread. The thread
							view link can be shared with other people.
						</small></p>
					</div>
				</div>
			</div>
			<div className="form-row justify-content-center">
				<div className="col-auto">
					<div className="form-group">
						<button
							type="button"
							className="btn btn-info"
							data-toggle="collapse"
							data-target="#submit-help-block"
						>
							Help
						</button>
					</div>
				</div>
				<div className="col-auto">
					<div className="form-group">
						<button
							type="submit"
							className={submitClass}
							onClick={this.submitId}
							disabled={!isValid}
						>
							Submit
						</button>
					</div>
				</div>
			</div>
		</form>
	}
}


export default class MainPage extends React.PureComponent {
	static propTypes = {
		navigate: PropTypes.func.isRequired,
	}

	// TODO: load BEFORE redirecting
	redirectToThread = tweetId => {
		this.props.navigate(`/thread/${tweetId}`)
	}

	render() {
		return <div className="container" id="homepage">
			<Title>Bobbin</Title>
			<div className="row">
				<div className="col text-center">
					<h2>Share threads with <strong>Bobbin</strong></h2>
				</div>
			</div>
			<div className="row">
				<div className="col">
					<TweetEntryForm submit={this.redirectToThread}/>
				</div>
			</div>
		</div>
	}
}
