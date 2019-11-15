package api

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"strconv"
	"strings"

	"github.com/Lucretiel/bobbin/twitter/auth"
)

type TweetId int64
type UserId int64

type User struct {
	Id          UserId
	Handle      string
	DisplayName string
}

type Tweet struct {
	User         User
	ParentId     *TweetId
	ParentUserId *UserId
}

type Tweets map[TweetId]Tweet

func (t Tweets) Merge(incoming Tweets) {
	for id, tweet := range incoming {
		t[id] = tweet
	}
}

type TweetIds map[TweetId]struct{}

func SingleTweetId(id TweetId) TweetIds {
	TweetIds{id: struct{}{}}
}

func GetTweets(
	ctx context.Context,
	client *http.Client,
	token auth.Token,
	tweets TweetIds,
) (
	Tweets, error,
) {
	request, err := http.NewRequestWithContext(ctx, "GET", "https://api.twitter.com/1.1/statuses/lookup.json", nil)
	if err != nil {
		return nil, err
	}

	// Add headers
	header := request.Header
	header.Set("Accept", "application/json")
	header.Set("Accept-Charset", "utf-8")
	token.AuthorizeRequest(request)

	// Comma separate the tweets
	var builder strings.Builder
	for tweetId := range tweets {
		fmt.Fprintf(&builder, "%v,", tweetId)
	}

	// Construct the query
	query := url.Values{}
	query.Add("id", builder.String())
	query.Add("include_entities", "false")
	query.Add("trim_user", "false")
	query.Add("map", "false")
	query.Add("include_ext_alt_text", "false")
	query.Add("include_card_uri", "false")

	request.URL.RawQuery = query.Encode()

	response, err := client.Do(request)
	if err != nil {
		return nil, err
	}

	defer response.Body.Close()

	if response.StatusCode > 200 {
		// TODO: special error if auth error, so a token refresh can be
		// attempted
		return nil, fmt.Errorf("Twitter API returned an error")
	}

	var responseData []struct {
		id                    TweetId
		in_reply_to_status_id *TweetId
		in_reply_to_user_id   *UserId
		user                  struct {
			id          UserId
			name        string
			screen_name string
		}
	}

	jsonDecoder := json.NewDecoder(response.Body)
	err = jsonDecoder.Decode(&responseData)

	if err != nil {
		return nil, err
	}

	result := Tweets{}

	for _, tweet := range responseData {
		result[tweet.id] = Tweet{
			ParentId:     tweet.in_reply_to_status_id,
			ParentUserId: tweet.in_reply_to_user_id,
			User: User{
				Id:          tweet.user.id,
				DisplayName: tweet.user.name,
				Handle:      tweet.user.screen_name,
			},
		}
	}

	return result, nil
}

func GetTweet(
	ctx context.Context,
	client *http.Client,
	token auth.Token,
	id TweetId,
) (
	Tweet, error,
) {
	tweets, err := GetTweets(ctx, client, token, Tweets{id: struct{}{}})
	if err != nil {
		return Tweet{}, err
	}
	tweet, ok := tweets[id]
	if !ok {
		return Tweet{}, fmt.Errorf("Couldn't find tweet with id %v", id)
	}
	return tweet, nil
}

func GetUserTweets(
	ctx context.Context,
	client *http.Client,
	token auth.Token,
	userId UserId,
	maxTweet TweetId,
) (
	Tweets, error,
) {
	request, err := http.NewRequestWithContext(ctx, "GET", "https://api.twitter.com/1.1/statuses/user_timeline.json", nil)
	if err != nil {
		return nil, err
	}

	// Add headers
	header := request.Header
	header.Set("Accept", "application/json")
	header.Set("Accept-Charset", "utf-8")
	token.AuthorizeRequest(request)

	// Construct the query
	query := url.Values{}
	query.Add("user_id", strconv.FormatInt(int64(userId), 10))
	query.Add("max_id", strconv.FormatInt(int64(maxTweet), 10))
	query.Add("count", "200")
	query.Add("trim_user", "false")
	query.Add("exclude_replies", "false")
	query.Add("include_rts", "true")

	request.URL.RawQuery = query.Encode()

	response, err := client.Do(request)
	if err != nil {
		return nil, err
	}

	defer response.Body.Close()

	if response.StatusCode > 200 {
		// TODO: special error if auth error, so a token refresh can be
		// attempted
		return nil, fmt.Errorf("Twitter API returned an error")
	}

	var responseData []struct {
		id                    TweetId
		in_reply_to_status_id *TweetId
		in_reply_to_user_id   *UserId
		user                  struct {
			id          UserId
			name        string
			screen_name string
		}
	}

	jsonDecoder := json.NewDecoder(response.Body)
	err = jsonDecoder.Decode(&responseData)

	if err != nil {
		return nil, err
	}

	result := Tweets{}

	for _, tweet := range responseData {
		result[tweet.id] = Tweet{
			ParentId:     tweet.in_reply_to_status_id,
			ParentUserId: tweet.in_reply_to_user_id,
			User: User{
				Id:          tweet.user.id,
				DisplayName: tweet.user.name,
				Handle:      tweet.user.screen_name,
			},
		}
	}

	return result, nil
}
