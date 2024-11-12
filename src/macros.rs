pub trait StrEnum: Sized {
    fn to_str(self) -> &'static str;
    fn from_str(s: &str) -> Option<Self>;
}

macro_rules! str_enum {
    ($( #[$attrs:meta] )* $vis:vis enum $name:ident {
        $( $( #[$variant_attrs:meta] )* $variant:ident = $str_value:literal ),* $(,)?
    }) => {
        $(#[$attrs])* $vis enum $name {
            $(
                $(#[$variant_attrs])* $variant
            ),*
        }
        impl $crate::macros::StrEnum for $name {
            fn to_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $str_value,)*
                }
            }

            fn from_str(s: &str) -> Option<Self> {
                match s {
                    $($str_value => Some(Self::$variant),)*
                    _ => None,
                }
            }
        }
    }
}
