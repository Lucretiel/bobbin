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
pub enum Script {
    Script {
        src: &'static str,
        asinc: bool,
        defer: bool,
    },
    Module {
        src: &'static str,
    },
}

impl Render for Script {
    fn render<'a>(&self, tmpl: &mut TemplateBuffer<'a>) {
        match *self {
            Script::Script { src, asinc, defer } => {
                tmpl << html! {
                    script(
                        src = src,
                        async ?= asinc,
                        defer ?= defer,
                        charset = "utf-8"
                    )
                }
            }
            Script::Module { src } => {
                tmpl << html! {
                       script(
                           src = src,
                           type = "module"
                       )
                }
            }
        }
    }
}

impl RenderMut for Script {
    fn render_mut<'a>(&mut self, tmpl: &mut TemplateBuffer<'a>) {
        self.render(tmpl)
    }
}

impl RenderOnce for Script {
    fn render_once(self, tmpl: &mut TemplateBuffer<'_>)
    where
        Self: Sized,
    {
        self.render(tmpl)
    }
}
