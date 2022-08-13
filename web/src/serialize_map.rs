/*!
A static, serializable map. Useful for sending static key-value data through
HTTP requests.

TODO: Make a version of this macro that support capturing local variable, in
addition to literals
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

#[macro_export]
macro_rules! serialize_map {

    (
        $($key:ident: $value:tt,)*
    ) => {{
        #[allow(non_camel_case_types)]
        struct SerializeLocalMap<$($key: serde::Serialize,)*> {$(
            $key: $key,
        )*}

        #[allow(non_camel_case_types)]
        impl<$($key: serde::Serialize,)*> serde::Serialize for SerializeLocalMap<$($key,)*> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                use serde::ser::SerializeStruct;

                let mut map = serializer.serialize_struct(
                    "",
                    $({ stringify!($key); 1 } +)* 0
                )?;

                $(
                    map.serialize_field(stringify!($key), &self.$key)?;
                )*

                map.end()
            }
        }

        SerializeLocalMap {$(
            $key: $crate::instantiate_serializable!($value),
        )*}
    }}
}

/// Helper macro for serialize_map. Turns literals into zero-size types.
#[macro_export]
macro_rules! instantiate_serializable {
    (true) => {
        $crate::instantiate_serializable!(@ true)
    };

    (false) => {
        $crate::instantiate_serializable!(@ false)
    };

    (None) => {
        $crate::instantiate_serializable!(@ ())
    };

    ($variable:ident) => {
        &$variable
    };

    ($literal:literal) => {
        $crate::instantiate_serializable!(@ $literal)
    };

    (@ $literal:expr) => {{
        struct Literal;

        impl serde::Serialize for Literal {
            #[inline]
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serde::Serialize::serialize(&$literal, serializer)
            }
        }

        Literal
    }};
}
