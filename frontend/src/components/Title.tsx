import React from 'react'

const Title: React.FC<{children: string}> = ({children}) => {
	React.useEffect(() => {
		document.title = children;
	});
	return null;
}

export default Title;
