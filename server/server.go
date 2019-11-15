package server

import (
	"bytes"
	"html/template"
	"net/http"
	"time"

	"github.com/julienschmidt/httprouter"
)

func plainTemplate(t *template.Template, name string) httprouter.Handle {
	now := time.Now()

	var buffer bytes.Buffer

	t.ExecuteTemplate(&buffer, name, nil)

	reader := bytes.NewReader(buffer.Bytes())

	return func(w http.ResponseWriter, r *http.Request, p httprouter.Params) {
		http.ServeContent(w, r, name, now, reader)
	}
}

type faqEntry struct {
	Slug, Question string
	Answer         template.HTML
}

var faqEntries []faqEntry = []faqEntry{
	{Slug: "A", Question: "B", Answer: template.HTML("C")},
}

func faqHandler() httprouter.Handle {
	now := time.Now()

	faqTemplate = template.Must(template.ParseFiles(
		"./templates/base.html",
		"./templates/faq.html",
	))

	var buffer bytes.Buffer

	t.ExecuteTemplate(&buffer, name, faqEntries)

	reader := bytes.NewReader(buffer.Bytes())

	return func(w http.ResponseWriter, r *http.Request, p httprouter.Params) {
		http.ServeContent(w, r, name, now, reader)
	}
}

func handleThread() httprouter.Handle {
	return func(w http.ResponseWriter, r *http.Request, p httprouter.Params) {

	}
}

// TODO: pass twitter API credentials into here
// TODO: inject templates?
func MakeRoutes() http.Handler {
	router := httprouter.New()
	router.GET("/", plainTemplate(templates, "index.html"))
	router.GET("/faq", faqHandler())
	router.GET("/thread/:id", handleThread())
	router.ServeFiles("/static/*filepath", http.Dir("./static"))

	return router
}
