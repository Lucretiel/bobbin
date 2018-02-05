const webpack = require('webpack')
const path = require('path')

const dir = local => path.resolve(__dirname, local)
const isProd = process.env.NODE_ENV === 'production'

module.exports = {
	context: dir("frontend-src"),
	entry: [
		'main.jsx',
		'style.scss',
	],
	output: {
		path: dir("dist/"),
		filename: 'bundle.js',
	},
	resolve: {
		modules: [
			dir("frontend-src"),
			"node_modules",
		],
	},
	module: {
		rules: [
			// Babelify everything
			{
				test: /\.jsx?$/,
				exclude: dir('node_modules'),
				use: [{
					loader: 'babel-loader',
					options: {
						plugins: [
							'lodash',
							"transform-class-properties",
						],
						presets: ['react', 'env'],
					},
				}],
			}, {
				test: /\.scss$/,
				exclude: dir('node_modules'),
				use: [
					"style-loader",
					"css-loader",
					"sass-loader",
				]
			}
		],
	},
}
