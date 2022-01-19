// f: facebook / opengraph
// t: twitter
// s: social (aka generic over facebook + twitter)
// m: meta (aka generic over facebook + twitter + meta) (this is just for description & title)
//
// TODO: clean this up. It's a challenge because the normal recursive macro thing
// doesn't work very well here, because we can't have arbitrary macro expansions
// inside of horrorshow.
#[macro_export]
macro_rules! social_tags {
    ($(
        $(f : $og_key:ident)?
        $(t : $twitter_key:ident)?
        $(m : $meta_key:ident)?
        $(s : $social_key:ident)?

        : $content:expr
    );* $(;)?) => {
        horrorshow::owned_html! {$(
            $( meta( property=concat!("og:", stringify!($og_key)), content=$content ); )?
            $( meta( name=concat!("twitter:", stringify!($twitter_key)), content=$content ); )?
            $(
                meta( property=concat!("og:", stringify!($social_key)), content=$content );
                meta( name=concat!("twitter:", stringify!($social_key)), content=$content );
            )?
            $(
                meta( name=stringify!($meta_key), content=$content );
                meta( property=concat!("og:", stringify!($meta_key)), content=$content );
                meta( name=concat!("twitter:", stringify!($meta_key)), content=$content );
            )?
        )*}
    };
}
