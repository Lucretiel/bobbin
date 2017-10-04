import React from 'react'
import PropTypes from 'prop-types'

export default class Title extends React.PureComponent {
	static propTypes = {
		children: PropTypes.string.isRequired,
	}

	constructor(props) {
		super(props)

		document.title = this.props.children
	}

	componentWillReceiveProps(newProps) {
		if(newProps.children != this.props.children) {
			document.title = newProps.children
		}
	}

	render() {
		return null
	}
}
