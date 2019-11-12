package auth

import (
	"context"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"strings"
)

type AppCredentials struct {
	encoded string
}

func (c Consumer) GetAppCredentials() TokenProducer {
	var builder strings.Builder
	builder.WriteString("Basic ")

	encoder := base64.NewEncoder(base64.StdEncoding, &builder)
	key := url.PathEscape(string(c.key))
	secret := url.PathEscape(string(c.secret))
	fmt.Fprintf(encoder, "%s:%s", key, secret)
	encoder.Close()

	return AppCredentials{
		encoded: builder.String(),
	}
}

type AppToken struct {
	token string
}

func (c AppCredentials) GetToken(ctx context.Context, client *http.Client) (Token, error) {
	request, err := http.NewRequestWithContext(ctx, "POST", "https://api.twitter.com/oauth2/token", strings.NewReader("grant_type=client_credentials"))
	if err != nil {
		return nil, err
	}

	header := request.Header
	header.Set("Authorization", c.encoded)
	header.Set("Content-Type", "application/x-www-form-urlencoded;charset=UTF-8")
	header.Set("Accept", "application/json")
	header.Set("Accept-Charset", "utf-8")

	response, err := client.Do(request)
	if err != nil {
		return nil, err
	}

	defer response.Body.Close()

	var responseData struct {
		token_type, access_token string
	}

	jsonDecoder := json.NewDecoder(response.Body)
	err = jsonDecoder.Decode(&responseData)

	if err != nil {
		return nil, err
	}

	if responseData.token_type != "bearer" {
		return nil, fmt.Errorf("Got an invalid token type from twitter (expected bearer): %s", responseData.token_type)
	}

	return AppToken{
		token: fmt.Sprintf("Bearer %s", responseData.access_token),
	}, nil
}

func (c AppToken) AuthorizeRequest(request *http.Request) {
	request.Header.Set("Authorization", c.token)
}
