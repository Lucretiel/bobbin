package twitter

import "github.com/Lucretiel/bobbin/twitter/api"

type ThreadBox interface {
	AddTweets(tweets api.Tweets)
}
