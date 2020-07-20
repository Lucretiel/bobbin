use horrorshow::html;
use horrorshow::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct Stylesheet {
    pub href: &'static str,
    pub integrity: Option<&'static str>,
    pub crossorigin: Option<&'static str>,
}

impl Stylesheet {
    pub const fn new(href: &'static str) -> Self {
        Self {
            href,
            integrity: None,
            crossorigin: None,
        }
    }
}

impl Render for Stylesheet {
    fn render<'a>(&self, tmpl: &mut TemplateBuffer<'a>) {
        tmpl << html! {
            link(
                rel = "stylesheet",
                href = self.href,
                integrity ?= self.integrity,
                crossorigin ?= self.crossorigin
            )
        }
    }
}

impl RenderMut for Stylesheet {
    fn render_mut<'a>(&mut self, tmpl: &mut TemplateBuffer<'a>) {
        self.render(tmpl)
    }
}

impl RenderOnce for Stylesheet {
    fn render_once(self, tmpl: &mut TemplateBuffer<'_>)
    where
        Self: Sized,
    {
        self.render(tmpl)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Module {
    pub src: &'static str,
}

impl Render for Module {
    fn render<'a>(&self, tmpl: &mut TemplateBuffer<'a>) {
        tmpl << html! {
            script(
                src = self.src,
                type = "module"
            )
        }
    }
}

impl RenderMut for Module {
    fn render_mut<'a>(&mut self, tmpl: &mut TemplateBuffer<'a>) {
        self.render(tmpl)
    }
}

impl RenderOnce for Module {
    fn render_once(self, tmpl: &mut TemplateBuffer<'_>)
    where
        Self: Sized,
    {
        self.render(tmpl)
    }
}
