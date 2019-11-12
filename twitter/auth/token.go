package auth

import (
	"context"
	"net/http"
)

type ConsumerKey string
type ConsumerSecret string

type Consumer struct {
	key    ConsumerKey
	secret ConsumerSecret
}

type TokenProducer interface {
	GetToken(ctx context.Context, client *http.Client) (Token, error)
}

type Token interface {
	AuthorizeRequest(request *http.Request)
}
