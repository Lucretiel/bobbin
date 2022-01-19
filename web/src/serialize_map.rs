/*!
A static, serializable map. Useful for sending static key-value data through
HTTP requests
*/

#[macro_export]
macro_rules! serialize_static_map {
    ($(
        $key:ident: $value:expr,
    )*) => {{
        struct SerializeLocalMap;

        impl serde::Serialize for SerializeLocalMap {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                use serde::ser::SerializeMap;

                let mut map = serializer.serialize_map(Some( $({ stringify!($key); 1 } +)* 0 ))?;

                $(
                    map.serialize_entry(stringify!($key), &$value)?;
                )*

                map.end()
            }
        }

        SerializeLocalMap
    }};
}
