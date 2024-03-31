macro_rules! str_enum {
    ($( #[$attrs:meta] )* $vis:vis enum $name:ident {
        $( $( #[$variant_attrs:meta] )* $variant:ident = $str_value:literal ),* $(,)?
    }) => {
        $(#[$attrs])* $vis enum $name {
            $(
                $(#[$variant_attrs])* $variant
            ),*
        }
        impl $name {
            $vis fn to_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $str_value,)*
                }
            }

            $vis fn from_str(s: &str) -> Option<Self> {
                match s {
                    $($str_value => Some(Self::$variant),)*
                    _ => None,
                }
            }
        }
    }
}
