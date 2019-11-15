package twitter

import (
	"context"
	"net/http"

	"github.com/Lucretiel/bobbin/twitter/api"
	"github.com/Lucretiel/bobbin/twitter/auth"
)

//
func GenerateThread(ctx context.Context, client *http.Client, token auth.Token, tail api.TweetId) (Tweets, err) {
	localStore := api.Tweets{}

	getTweet := func(id api.TweetId) (api.Tweet, error) {
		tweet, ok := localStore[id]
		if ok {
			return tweet, nil
		} else {
			// TODO: global cache
			// TODO: handle deleted / hidden / etc
			// TODO: data loader
			tweet, err := api.GetTweet(ctx, client, token, currentTweetId)
			if err != nil {
				return nil, err
			}

			if tweet.ParentId != nil {
				// No need to store this in localstore, but we should globally cache it
				user_tweets, err := api.GetUserTweets(ctx, client, token, *tweet.ParentUserId, id)

				// TODO: some errors here should be recoverable
				if err != nil {
					return nil, err
				}

				localStore.Merge(user_tweets)
			}

			return tweet, nil
		}
	}

	result := api.Tweets{}
	currentTweetId := tail

	for {
		tweet, err := getTweet(currentTweetId)
		if err != nil {
			return nil, err
		}
		result[currentTweetId] = tweet
		if tweet.ParentId == nil {
			return result, nil
		}
		currentTweetId = *tweet.ParentId
	}
}
