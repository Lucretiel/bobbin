/*
 * Copyright 2019 Nathan West
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

import React from 'react'

const Title: React.FC<{children: string}> = ({children}) => {
	React.useEffect(() => {
		document.title = children;
	});
	return null;
}

export default Title;
