#!/bin/bash

set -e

echo '/* eslint-disable */'
echo
curl "https://platform.twitter.com/widgets.js"
echo
echo
echo 'export default window.twttr;'
